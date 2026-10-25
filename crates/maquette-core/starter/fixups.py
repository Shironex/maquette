"""Fix-ups and real size: repairs recorded per asset, replayed on every version.

The fix-ups and the real size of each asset live in prepare.json, keyed by
asset id. They are applied in this order, whatever order they were added in:

    apply-transforms   bake object transforms into the meshes
    remove-loose       drop loose parts smaller than "minShare" of the model
    merge              join the meshes into one ("byMaterial": one per material)
    decimate           collapse down to "triangles"
    shrink-textures    scale images down to "maxSize" px on the longest edge
    origin-bottom      put the origin at the bottom centre, on the floor

then the real size: a uniform scale so the model's "height", "width",
"depth" or "longest" side measures "value" in "unit" (cm or m). The scene
layout's own scale applies on top of that.

Maquette uses this file in two ways, always headless:

1. On its own, to show what the fix-ups do before you keep them:

    blender -b --factory-startup --python-exit-code 1 --python tools/fixups.py -- \
        --input model.glb --plan plan.json --output fixed.glb --stats stats.json

   plan.json is one asset's entry ({"size": ..., "fixups": [...]}). The
   input file is only read; the fixed model goes to --output and the numbers
   before and after every fix-up to --stats.

2. From build_scene.py, which imports it and calls prepare() on the objects
   of each model it imports, before the layout moves them.

Rigged meshes keep their skin: transforms are not applied to them and
merge leaves them apart. Edit freely: this file is yours. Maquette reruns
the Build scene step when it or prepare.json changes.
"""

import argparse
import json
import math
import os
import sys

import bmesh
import bpy
from mathutils import Matrix, Vector

PREPARE_VERSION = 1
ORDER = [
    "apply-transforms",
    "remove-loose",
    "merge",
    "decimate",
    "shrink-textures",
    "origin-bottom",
]
UNIT_METRES = {"cm": 0.01, "m": 1.0}


def log(message):
    print(f"[fixups] {message}", flush=True)


def live(objects):
    """The objects not deleted yet. A deleted one raises on any access."""
    found = []
    for obj in objects:
        try:
            if obj.name in bpy.data.objects:
                found.append(obj)
        except ReferenceError:
            pass
    return found


def meshes(objects):
    return [o for o in live(objects) if o.type == "MESH"]


def rigged(obj):
    return obj.find_armature() is not None or any(m.type == "ARMATURE" for m in obj.modifiers)


def triangles(obj):
    mesh = obj.data
    mesh.calc_loop_triangles()
    return len(mesh.loop_triangles)


def images(objects):
    found = []
    for obj in meshes(objects):
        for slot in obj.material_slots:
            material = slot.material
            if material is None or material.node_tree is None:
                continue
            for node in material.node_tree.nodes:
                if node.type == "TEX_IMAGE" and node.image is not None and node.image not in found:
                    found.append(node.image)
    return found


def world_bounds(objects):
    """Blender space min and max corners of the meshes, or None without any."""
    lo = Vector((math.inf,) * 3)
    hi = Vector((-math.inf,) * 3)
    for obj in meshes(objects):
        for vertex in obj.data.vertices:
            point = obj.matrix_world @ vertex.co
            lo = Vector(map(min, lo, point))
            hi = Vector(map(max, hi, point))
    if lo.x == math.inf:
        return None
    return lo, hi


def gltf_bounds(objects):
    """[min, max] in glTF axes (Y up): x stays, y is Blender z, z is -y."""
    found = world_bounds(objects)
    if found is None:
        return None
    lo, hi = found
    return [[lo.x, lo.z, -hi.y], [hi.x, hi.z, -lo.y]]


def stats(objects):
    bounds = gltf_bounds(objects)
    edges = [max(image.size[0], image.size[1]) for image in images(objects)]
    return {
        "triangles": sum(triangles(o) for o in meshes(objects)),
        "objects": len(meshes(objects)),
        "maxTexture": max(edges, default=0),
        "min": [round(v, 6) for v in bounds[0]] if bounds else None,
        "max": [round(v, 6) for v in bounds[1]] if bounds else None,
    }


def roots(objects):
    inside = set(objects)
    return [o for o in objects if o.parent is None or o.parent not in inside]


def single_user(obj):
    if obj.data is not None and obj.data.users > 1:
        obj.data = obj.data.copy()


def apply_transforms(objects, _fixup):
    done = 0
    for obj in meshes(objects):
        if rigged(obj):
            continue
        single_user(obj)
        world = obj.matrix_world.copy()
        obj.parent = None
        obj.data.transform(world)
        obj.matrix_world = Matrix.Identity(4)
        done += 1
    skipped = len(meshes(objects)) - done
    # Empties left with nothing under them only carried those transforms.
    for obj in [o for o in live(objects) if o.type == "EMPTY"]:
        if not obj.children:
            bpy.data.objects.remove(obj)
    return f"{done} meshes, {skipped} rigged kept" if skipped else f"{done} meshes"


def components(bm):
    """Groups of vertices that touch: joined by an edge or at the same spot,
    so a face split off along a UV seam still counts as part of its object."""
    parent = list(range(len(bm.verts)))

    def find(i):
        while parent[i] != i:
            parent[i] = parent[parent[i]]
            i = parent[i]
        return i

    def union(a, b):
        ra, rb = find(a), find(b)
        if ra != rb:
            parent[ra] = rb

    for edge in bm.edges:
        union(edge.verts[0].index, edge.verts[1].index)
    spots = {}
    for vert in bm.verts:
        key = tuple(round(c, 5) for c in vert.co)
        if key in spots:
            union(vert.index, spots[key])
        else:
            spots[key] = vert.index
    groups = {}
    for vert in bm.verts:
        groups.setdefault(find(vert.index), []).append(vert)
    return list(groups.values())


def diagonal(points):
    lo = Vector(map(min, *points)) if len(points) > 1 else points[0]
    hi = Vector(map(max, *points)) if len(points) > 1 else points[0]
    return (hi - lo).length


def remove_loose(objects, fixup):
    found = world_bounds(objects)
    if found is None:
        return "no meshes"
    limit = float(fixup.get("minShare", 0.02)) * (found[1] - found[0]).length
    parts = 0
    for obj in meshes(objects):
        single_user(obj)
        bm = bmesh.new()
        bm.from_mesh(obj.data)
        bm.verts.ensure_lookup_table()
        bm.verts.index_update()
        drop = []
        for group in components(bm):
            points = [obj.matrix_world @ v.co for v in group]
            if diagonal(points) < limit:
                drop.extend(group)
                parts += 1
        if drop:
            bmesh.ops.delete(bm, geom=drop, context="VERTS")
            bm.to_mesh(obj.data)
        bm.free()
    for obj in meshes(objects):
        if len(obj.data.vertices) == 0:
            bpy.data.objects.remove(obj)
    return f"{parts} {'part' if parts == 1 else 'parts'} under {limit:.4g} m"


def join(group):
    if len(group) < 2:
        return 0
    active = group[0]
    with bpy.context.temp_override(
        active_object=active,
        object=active,
        selected_objects=group,
        selected_editable_objects=group,
    ):
        bpy.ops.object.join()
    return len(group) - 1


def merge(objects, fixup):
    loose = [o for o in meshes(objects) if not rigged(o)]
    if fixup.get("byMaterial"):
        by_look = {}
        for obj in loose:
            key = tuple(sorted(s.material.name for s in obj.material_slots if s.material))
            by_look.setdefault(key, []).append(obj)
        groups = list(by_look.values())
    else:
        groups = [loose]
    joined = sum(join(group) for group in groups)
    if joined == 0:
        return "nothing to join"
    return f"{len(loose)} meshes joined into {len(groups)}"


def decimate(objects, fixup):
    target = int(fixup.get("triangles", 0))
    total = sum(triangles(o) for o in meshes(objects))
    if target <= 0 or total <= target:
        return f"already {total} triangles"
    ratio = target / total
    skipped = 0
    for obj in meshes(objects):
        if obj.data.shape_keys is not None:
            skipped += 1
            continue
        single_user(obj)
        # Only the decimate is evaluated: a skin or other modifier stays live.
        others = [m for m in obj.modifiers if m.show_viewport]
        for modifier in others:
            modifier.show_viewport = False
        modifier = obj.modifiers.new("maquette_decimate", "DECIMATE")
        modifier.decimate_type = "COLLAPSE"
        modifier.ratio = ratio
        depsgraph = bpy.context.evaluated_depsgraph_get()
        depsgraph.update()
        evaluated = obj.evaluated_get(depsgraph)
        mesh = bpy.data.meshes.new_from_object(
            evaluated, preserve_all_data_layers=True, depsgraph=depsgraph
        )
        obj.modifiers.remove(modifier)
        for modifier in others:
            modifier.show_viewport = True
        old = obj.data
        obj.data = mesh
        if old.users == 0:
            bpy.data.meshes.remove(old)
    note = f"ratio {ratio:.3f}"
    return f"{note}, {skipped} meshes with shape keys kept" if skipped else note


def shrink_textures(objects, fixup):
    limit = int(fixup.get("maxSize", 2048))
    shrunk = 0
    for image in images(objects):
        width, height = image.size
        edge = max(width, height)
        if edge <= limit or edge == 0:
            continue
        factor = limit / edge
        image.scale(max(1, round(width * factor)), max(1, round(height * factor)))
        # The exporter writes packed bytes as they are, so pack the scaled pixels.
        image.pack()
        shrunk += 1
    return f"{shrunk} textures to {limit} px"


def origin_bottom(objects, _fixup):
    found = world_bounds(objects)
    if found is None:
        return "no meshes"
    lo, hi = found
    shift = Vector(((lo.x + hi.x) / 2, (lo.y + hi.y) / 2, lo.z))
    for obj in roots(objects):
        obj.matrix_world = Matrix.Translation(-shift) @ obj.matrix_world
    return f"moved by {-shift.x:.4g}, {-shift.z:.4g}, {shift.y:.4g} m"


OPS = {
    "apply-transforms": apply_transforms,
    "remove-loose": remove_loose,
    "merge": merge,
    "decimate": decimate,
    "shrink-textures": shrink_textures,
    "origin-bottom": origin_bottom,
}


def size_scale(objects, size):
    """The uniform scale that gives the model its real size, or None."""
    bounds = gltf_bounds(objects)
    if not size or bounds is None:
        return None
    extent = [hi - lo for lo, hi in zip(bounds[0], bounds[1])]
    axis = size.get("axis", "longest")
    measured = {"width": extent[0], "height": extent[1], "depth": extent[2]}.get(
        axis, max(extent)
    )
    target = float(size.get("value", 0)) * UNIT_METRES.get(size.get("unit", "cm"), 0.01)
    if measured <= 1e-9 or target <= 0:
        return None
    return target / measured


def prepare(objects, entry, say=log):
    """Applies one asset's fix-ups, then its real size, to its objects.

    Returns the objects that are left and one report per fix-up."""
    objects = live(objects)
    steps = []
    wanted = {f.get("op"): f for f in entry.get("fixups") or []}
    for op in ORDER:
        if op not in wanted:
            continue
        before = stats(objects)
        bpy.context.view_layer.update()
        note = OPS[op](objects, wanted[op])
        bpy.context.view_layer.update()
        # Merge and remove-loose delete objects; none of them adds one.
        objects = live(objects)
        after = stats(objects)
        say(f"{op}: {note}")
        steps.append({"op": op, "before": before, "after": after, "note": note})
    unknown = sorted(set(wanted) - set(ORDER))
    if unknown:
        say(f"skipped unknown fix-ups: {', '.join(str(u) for u in unknown)}")
    scale = size_scale(objects, entry.get("size"))
    if scale is not None:
        for obj in roots(objects):
            obj.matrix_world = Matrix.Scale(scale, 4) @ obj.matrix_world
        bpy.context.view_layer.update()
        say(f"real size: scaled by {scale:.5g}")
    return objects, steps, scale


def read_entries(path):
    """prepare.json as asset id to entry; empty when missing or newer."""
    if not path or not os.path.isfile(path):
        return {}
    try:
        with open(path, encoding="utf-8") as fh:
            found = json.load(fh)
    except (OSError, ValueError) as err:
        log(f"could not read {path}: {err}")
        return {}
    if found.get("version", 1) > PREPARE_VERSION:
        log(f"{path} comes from a newer Maquette, so it is left out")
        return {}
    return found.get("models") or {}


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="fixups.py")
    parser.add_argument("--input", required=True, help="the model to read, never written")
    parser.add_argument("--plan", required=True, help="one asset's size and fix-ups, as JSON")
    parser.add_argument("--output", required=True, help="where the fixed GLB goes")
    parser.add_argument("--stats", required=True, help="where the numbers go, as JSON")
    return parser.parse_args(argv)


def main():
    args = parse_args()
    with open(args.plan, encoding="utf-8") as fh:
        plan = json.load(fh)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    try:
        bpy.ops.import_scene.gltf(filepath=os.path.abspath(args.input), disable_bone_shape=True)
    except TypeError:  # an importer without the option
        bpy.ops.import_scene.gltf(filepath=os.path.abspath(args.input))
    objects = list(bpy.data.objects)
    bpy.context.view_layer.update()
    before = stats(objects)
    objects, steps, scale = prepare(objects, plan)
    after = stats(objects)
    out = os.path.abspath(args.output)
    os.makedirs(os.path.dirname(out), exist_ok=True)
    bpy.ops.export_scene.gltf(filepath=out, export_format="GLB")
    with open(args.stats, "w", encoding="utf-8") as fh:
        json.dump({"before": before, "after": after, "steps": steps, "scale": scale}, fh, indent=2)
    log(f"wrote {args.output}: {before['triangles']} to {after['triangles']} triangles")


if __name__ == "__main__":
    main()
