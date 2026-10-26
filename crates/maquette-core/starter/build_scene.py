"""Build scene: put every model from the models folder into one Blender scene.

Maquette runs this headless, as the Build scene step:

    blender -b --factory-startup --python-exit-code 1 --python tools/build_scene.py -- \
        --base blender/scene.blend --models models --out blender/scene_built.blend \
        [--layout layout.json] [--project project.json] [--prepare prepare.json]

- When the base blend exists it is the starting point (your cameras, lights
  and hand placed objects). Otherwise the scene starts empty, with a sun and
  a grey world so the posters are lit.
- Each models/*.glb or *.gltf is imported into its own collection under
  "Models", named after the file. A model whose collection is already in the
  base blend is left as you placed it. When project.json lists a file as an
  older version of an asset, only the newest version is imported.
- Blender keeps names unique, so a second model's "palette" material or
  "lid" object comes in as "palette.001". Each renamed object and material
  is tagged "maquette_name" with the name from its file, and Export GLB
  writes that name back, so a pivot or a material an app looks up by name
  ships as the model has it.
- Each asset's fix-ups and real size (prepare.json, set in the Inbox, the
  Board and the Inspector) are applied by fixups.py next to this script:
  the fix-ups, then a uniform scale to the real size. The layout comes
  after, so its "scale" stays a tweak on top of the real size.
- An asset set to skip the bake ("skipBake": true in prepare.json, the
  default for a model accepted from the Inbox) has every object tagged
  "maquette_skip_bake": Bake light leaves it out, so it ships with its own
  material lighting, no lightmap and no second UV set.
- The scene layout (layout.json, written by the Inspector's Layout view)
  moves each asset's newest model to its place: "position" in metres,
  "rotation" in degrees (X, then Y, then Z) and a uniform "scale", all in
  glTF coordinates (Y up), keyed by asset id. Models without a placement
  that overlap at the origin are laid out side by side along X.
- Its "desk" becomes a plain slab named "desk" under "Models" (top at the
  floor, "size" is width and depth in metres), so the models cast their
  shadows on it in the bake and it ships with them. When a model already
  has a floor there (it reaches below the desk top over most of the slab),
  the slab is left out: a second surface at the same height would cover
  that floor and bake it black.
- Its "clips" say which animation clips of a model stay off the site
  ({"<asset>": {"<clip>": {"ship": false}}}, set in the Motion view): each
  such clip is removed from the model's objects, so Export GLB leaves it
  out. Every other clip ships.
- Its "stops" become cameras in a "Stops" collection, named
  stop_01_<name> and tagged "maquette_stop" (the order) and
  "maquette_stop_name", which Capture posters renders. When the base blend
  already has cameras tagged or named as stops, those are kept and the
  layout's stops are left out.
- The result is saved to --out. The base blend is never written.

Edit freely: this file is yours. Maquette only reruns the step when this
script, fixups.py, the base blend, a model, the layout or prepare.json
changes.
"""

import argparse
import glob
import json
import math
import os
import re
import struct
import sys

import bpy
from mathutils import Matrix, Vector

MODELS = "Models"
STUDIO = "Studio"
STOPS = "Stops"
DESK = "desk"
DESK_THICKNESS = 0.04
# A model that reaches this far below the desk top over this share of the
# slab is a floor of its own.
FLOOR_DEPTH = 0.005
FLOOR_SHARE = 0.5
SKIP_BAKE = "maquette_skip_bake"
WORLD_GREY = 0.18
TAG = "maquette_stop"
SOURCE_NAME = "maquette_name"
SUFFIX = re.compile(r"^(.*)\.\d{3,}$")
LAYOUT_VERSION = 1
# glTF (Y up) to Blender (Z up): x stays, y becomes z, z becomes -y.
GLTF_TO_BLENDER = Matrix(((1, 0, 0, 0), (0, 0, -1, 0), (0, 1, 0, 0), (0, 0, 0, 1)))


def log(message):
    print(f"[build_scene] {message}", flush=True)


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="build_scene.py")
    parser.add_argument("--base", default="blender/scene.blend", help="optional starting blend")
    parser.add_argument("--models", default="models", help="folder with .glb or .gltf files")
    parser.add_argument("--out", required=True, help="where to save the built blend")
    parser.add_argument("--layout", default="layout.json", help="optional scene layout")
    parser.add_argument("--project", default="project.json", help="the project's assets")
    parser.add_argument("--prepare", default="prepare.json", help="real sizes and fix-ups")
    return parser.parse_args(argv)


def snake(name):
    text = re.sub(r"[^0-9a-zA-Z]+", "_", name).strip("_").lower()
    return text or "stop"


def read_json(path, what):
    if not os.path.isfile(path):
        return None
    try:
        with open(path, encoding="utf-8") as fh:
            return json.load(fh)
    except (OSError, ValueError) as err:
        log(f"could not read {what} {path}: {err}")
        return None


def read_layout(path):
    layout = read_json(path, "the layout")
    if layout is None:
        return {}
    if layout.get("version", 1) > LAYOUT_VERSION:
        log(f"{path} comes from a newer Maquette, so it is left out")
        return {}
    return layout


def asset_files(path):
    """File stem to (asset id, newest), from project.json."""
    project = read_json(path, "the project") or {}
    found = {}
    for asset in project.get("assets", []):
        versions = asset.get("versions") or []
        for i, version in enumerate(versions):
            stem = os.path.splitext(os.path.basename(version.get("file", "")))[0]
            found[stem] = (asset.get("id"), i == len(versions) - 1)
    return found


def load_fixups():
    """fixups.py from this script's folder, or None when it is missing."""
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    try:
        import fixups
    except ImportError as err:
        log(f"fixups.py is missing, so real sizes and fix-ups are left out ({err})")
        return None
    return fixups


def prepare_models(collections, owners, path):
    """Applies each asset's fix-ups and real size to its newest model, and
    tags the objects of an asset that skips the bake."""
    fixups = load_fixups()
    entries = fixups.read_entries(path) if fixups else read_prepare(path)
    skipped = 0
    for collection in collections:
        owner = owners.get(collection.name)
        entry = entries.get(owner[0]) if owner else None
        if not entry:
            continue
        if entry.get("skipBake") is True:
            for obj in collection.all_objects:
                obj[SKIP_BAKE] = True
            skipped += 1
        if fixups is None:
            continue
        say = lambda message, name=collection.name: log(f"{name} {message}")
        fixups.prepare(list(collection.all_objects), entry, say)
    bpy.context.view_layer.update()
    if skipped:
        log(f"{skipped} models skip the bake and keep their own material lighting")


def read_prepare(path):
    """prepare.json's entries when fixups.py is missing."""
    found = read_json(path, "the preparation") or {}
    return found.get("models") or {}


def start_scene(base):
    if os.path.isfile(base):
        log(f"starting from {base}")
        bpy.ops.wm.open_mainfile(filepath=os.path.abspath(base))
        return
    log(f"no {base} yet, starting from an empty scene")
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    studio = bpy.data.collections.new(STUDIO)
    scene.collection.children.link(studio)
    sun = bpy.data.objects.new("Sun", bpy.data.lights.new("Sun", type="SUN"))
    sun.data.energy = 3.0
    sun.rotation_euler = (0.9, 0.0, 0.8)
    studio.objects.link(sun)
    world = bpy.data.worlds.new("World")
    world.color = (WORLD_GREY, WORLD_GREY, WORLD_GREY)
    # Cycles and the bake read the world's nodes, not its viewport colour.
    if world.node_tree is None:
        world.use_nodes = True
    background = next((n for n in world.node_tree.nodes if n.type == "BACKGROUND"), None)
    if background is not None:
        background.inputs["Color"].default_value = (WORLD_GREY, WORLD_GREY, WORLD_GREY, 1.0)
    scene.world = world


def child_collection(scene, name):
    found = bpy.data.collections.get(name)
    if found is None:
        found = bpy.data.collections.new(name)
    if found.name not in scene.collection.children:
        scene.collection.children.link(found)
    return found


def gltf_names(path):
    """The node and material names a .glb or .gltf file gives."""
    try:
        with open(path, "rb") as fh:
            data = fh.read()
        if data[:4] == b"glTF":
            length = struct.unpack_from("<I", data, 12)[0]
            doc = json.loads(data[20 : 20 + length])
        else:
            doc = json.loads(data)
    except (OSError, ValueError, struct.error):
        return set(), set()
    nodes = {n.get("name") for n in doc.get("nodes", []) if n.get("name")}
    materials = {m.get("name") for m in doc.get("materials", []) if m.get("name")}
    return nodes, materials


def keep_source_names(path, objects):
    """Tags what Blender renamed on import with the name from the file."""
    nodes, materials = gltf_names(path)

    def original(name, names):
        if name in names:
            return None
        match = SUFFIX.match(name)
        return match.group(1) if match and match.group(1) in names else None

    tagged = 0
    for obj in objects:
        was = original(obj.name, nodes)
        if was:
            obj[SOURCE_NAME] = was
            tagged += 1
        for slot in getattr(obj, "material_slots", []):
            material = slot.material
            if material is None or SOURCE_NAME in material:
                continue
            was = original(material.name, materials)
            if was:
                material[SOURCE_NAME] = was
                tagged += 1
    if tagged:
        log(f"{os.path.basename(path)}: {tagged} names taken, Export GLB writes the file's own back")


def import_model(path, parent):
    """Imports one file into a new collection named after it."""
    name = os.path.splitext(os.path.basename(path))[0]
    if name in bpy.data.collections:
        log(f"{name} is already in the base scene, kept as placed")
        return None
    before = set(bpy.data.objects)
    try:
        # No icosphere bone shapes for rigged models: they would export as a mesh.
        bpy.ops.import_scene.gltf(filepath=os.path.abspath(path), disable_bone_shape=True)
    except TypeError:  # an importer without the option
        bpy.ops.import_scene.gltf(filepath=os.path.abspath(path))
    added = [o for o in bpy.data.objects if o not in before]
    keep_source_names(path, added)
    target = bpy.data.collections.new(name)
    parent.children.link(target)
    for obj in added:
        for owner in list(obj.users_collection):
            owner.objects.unlink(obj)
        target.objects.link(obj)
    log(f"imported {os.path.basename(path)}: {len(added)} objects")
    return target


def drop_clips(collection, names):
    """Removes the named clips (glTF animations, which import as actions and
    NLA tracks of the same name) from a model's objects, so the export
    leaves them out. Returns the names it found."""
    wanted = set(names)
    gone = set()
    actions = set()

    def named(action):
        if action is None:
            return False
        match = SUFFIX.match(action.name)
        return (match.group(1) if match else action.name) in wanted

    for obj in collection.all_objects:
        data = obj.animation_data
        if data is None:
            continue
        if named(data.action):
            actions.add(data.action)
            data.action = None
        for track in list(data.nla_tracks):
            for strip in list(track.strips):
                if named(strip.action) or track.name in wanted:
                    if strip.action is not None:
                        actions.add(strip.action)
                    track.strips.remove(strip)
            if len(track.strips) == 0:
                data.nla_tracks.remove(track)
    for action in actions:
        match = SUFFIX.match(action.name)
        gone.add(match.group(1) if match else action.name)
        bpy.data.actions.remove(action)
    return gone


def bounds(collection):
    """World space min and max corners of the meshes in a collection."""
    lo = Vector((float("inf"),) * 3)
    hi = Vector((float("-inf"),) * 3)
    for obj in collection.all_objects:
        if obj.type != "MESH":
            continue
        for corner in obj.bound_box:
            point = obj.matrix_world @ Vector(corner)
            lo = Vector(map(min, lo, point))
            hi = Vector(map(max, hi, point))
    if lo.x == float("inf"):
        return None
    return lo, hi


def overlaps(a, b):
    return all(a[0][i] < b[1][i] and b[0][i] < a[1][i] for i in range(3))


def lay_out(collections):
    """Moves the models into a row along X when any two of them overlap."""
    boxes = [(c, bounds(c)) for c in collections]
    boxes = [(c, b) for c, b in boxes if b is not None]
    if not any(overlaps(a[1], b[1]) for i, a in enumerate(boxes) for b in boxes[i + 1 :]):
        return
    gap = 0.25 * max((b[1] - b[0]).length for _, b in boxes)
    widths = [b[1].x - b[0].x for _, b in boxes]
    x = -(sum(widths) + gap * (len(boxes) - 1)) / 2
    for (collection, (lo, _hi)), width in zip(boxes, widths):
        shift = Vector((x - lo.x, 0.0, 0.0))
        for obj in collection.objects:
            if obj.parent is None:
                obj.location += shift
        x += width + gap
    bpy.context.view_layer.update()
    log(f"laid {len(boxes)} models out side by side")


def placement_matrix(placement):
    """The placement's glTF matrix (translate, rotate X Y Z, scale), in Blender space."""
    position = placement.get("position", [0, 0, 0])
    rx, ry, rz = (math.radians(v) for v in placement.get("rotation", [0, 0, 0]))
    scale = placement.get("scale", 1.0)
    rotation = (
        Matrix.Rotation(rx, 4, "X") @ Matrix.Rotation(ry, 4, "Y") @ Matrix.Rotation(rz, 4, "Z")
    )
    local = Matrix.Translation(Vector(position)) @ rotation @ Matrix.Scale(scale, 4)
    return GLTF_TO_BLENDER @ local @ GLTF_TO_BLENDER.inverted()


def place(collection, placement):
    move = placement_matrix(placement)
    for obj in collection.objects:
        if obj.parent is None:
            obj.matrix_world = move @ obj.matrix_world


def blender_point(v):
    return (GLTF_TO_BLENDER @ Vector((v[0], v[1], v[2], 1.0))).to_3d()


def covered_share(lo, hi, x, y):
    """How much of the slab (-x..x, -y..y) a box from lo to hi covers."""
    w = max(0.0, min(hi.x, x) - max(lo.x, -x))
    d = max(0.0, min(hi.y, y) - max(lo.y, -y))
    return (w * d) / (4 * x * y)


def floor_under(parent, x, y):
    """The name of a model mesh that is a floor at the desk top, or None:
    it reaches below the top and up to it, over most of the slab."""
    for obj in parent.all_objects:
        if obj.type != "MESH":
            continue
        corners = [obj.matrix_world @ Vector(c) for c in obj.bound_box]
        lo = Vector(map(min, *corners))
        hi = Vector(map(max, *corners))
        if lo.z > -FLOOR_DEPTH or hi.z < -FLOOR_DEPTH:
            continue
        if covered_share(lo, hi, x, y) >= FLOOR_SHARE:
            return obj.name
    return None


def add_desk(parent, desk):
    if DESK in bpy.data.collections:
        log("the base scene already has a desk, kept as placed")
        return
    width, depth = desk.get("size", [1.6, 0.8])
    floor = floor_under(parent, width / 2, depth / 2)
    if floor is not None:
        log(f"{floor} is already a floor at the desk top, so the layout's desk slab is left out")
        return
    mesh = bpy.data.meshes.new(DESK)
    x, y, t = width / 2, depth / 2, DESK_THICKNESS
    verts = [(sx * x, sy * y, z) for z in (-t, 0.0) for sy in (-1, 1) for sx in (-1, 1)]
    # Counter clockwise seen from outside, so every normal faces out.
    faces = [(0, 2, 3, 1), (4, 5, 7, 6), (0, 1, 5, 4), (2, 6, 7, 3), (0, 4, 6, 2), (1, 3, 7, 5)]
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    material = bpy.data.materials.new(f"{DESK} surface")
    if material.node_tree is None:
        material.use_nodes = True
    bsdf = material.node_tree.nodes.get("Principled BSDF")
    if bsdf is not None:
        bsdf.inputs["Base Color"].default_value = (0.32, 0.22, 0.15, 1.0)
        bsdf.inputs["Roughness"].default_value = 0.75
    mesh.materials.append(material)
    obj = bpy.data.objects.new(DESK, mesh)
    target = bpy.data.collections.new(DESK)
    parent.children.link(target)
    target.objects.link(obj)
    log(f"added a {width:g} by {depth:g} m desk")


def own_stops(scene):
    return [
        o
        for o in scene.objects
        if o.type == "CAMERA"
        and (o.get(TAG) or o.name.startswith("stop_") or o.name.startswith("Stop "))
    ]


def add_stops(scene, stops):
    mine = own_stops(scene)
    if mine:
        log(f"the base scene has {len(mine)} camera stops of its own, so the layout's {len(stops)} are left out")
        return
    collection = child_collection(scene, STOPS)
    for i, stop in enumerate(stops):
        name = stop.get("name") or f"Stop {i + 1}"
        data = bpy.data.cameras.new(f"stop_{i + 1:02d}_{snake(name)}")
        data.sensor_fit = "VERTICAL"
        data.angle_y = math.radians(stop.get("fov", 35.0))
        data.clip_start = 0.01
        data.clip_end = 1000.0
        camera = bpy.data.objects.new(data.name, data)
        position = blender_point(stop["position"])
        target = blender_point(stop["target"])
        camera.location = position
        camera.rotation_euler = (target - position).to_track_quat("-Z", "Y").to_euler()
        camera[TAG] = i + 1
        camera["maquette_stop_name"] = name
        collection.objects.link(camera)
    log(f"added {len(stops)} camera stops from the layout")


def main():
    args = parse_args()
    layout = read_layout(args.layout)
    placements = layout.get("models") or {}
    owners = asset_files(args.project)
    start_scene(args.base)
    scene = bpy.context.scene
    parent = child_collection(scene, MODELS)
    files = sorted(glob.glob(os.path.join(args.models, "*.glb")))
    files += sorted(glob.glob(os.path.join(args.models, "*.gltf")))
    if not files:
        log(f"no .glb or .gltf files in {args.models}, saving the scene without models")
    newest = []
    for path in files:
        stem = os.path.splitext(os.path.basename(path))[0]
        owner = owners.get(stem)
        if owner is not None and not owner[1]:
            log(f"skipped {os.path.basename(path)}, an older version of {owner[0]}")
            continue
        newest.append(path)
    added = [c for c in (import_model(f, parent) for f in newest) if c is not None]
    bpy.context.view_layer.update()
    prepare_models(added, owners, args.prepare)
    for collection in added:
        owner = owners.get(collection.name)
        choices = (layout.get("clips") or {}).get(owner[0], {}) if owner else {}
        off = [name for name, choice in choices.items() if choice.get("ship") is False]
        if off:
            gone = drop_clips(collection, off)
            left = sorted(set(off) - gone)
            log(f"{collection.name}: left {len(gone)} clips off the site" + (f", no clip named {', '.join(left)}" if left else ""))
    unplaced = []
    placed = 0
    for collection in added:
        owner = owners.get(collection.name)
        placement = placements.get(owner[0]) if owner else None
        if placement is None:
            unplaced.append(collection)
        else:
            place(collection, placement)
            placed += 1
    bpy.context.view_layer.update()
    if placed:
        log(f"placed {placed} models from {args.layout}")
    if len(unplaced) > 1:
        lay_out(unplaced)
    if layout.get("desk"):
        add_desk(parent, layout["desk"])
    if layout.get("stops"):
        add_stops(scene, layout["stops"])
    out = os.path.abspath(args.out)
    os.makedirs(os.path.dirname(out), exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=out)
    log(f"saved {args.out} with {len(parent.children)} models")


main()
