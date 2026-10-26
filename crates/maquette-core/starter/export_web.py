"""Export GLB: one binary glTF per model, ready for the Compress step.

Maquette runs this headless, as the Export GLB step:

    blender -b --factory-startup --python-exit-code 1 blender/scene_baked.blend \
        --python tools/export_web.py -- --bake blender/bake --out web/raw

What becomes a file:
- each child collection of "Models" (one per imported model) is one GLB;
- each other top level mesh, empty or armature outside "Models" and
  "Studio" is one GLB with its children.
File names are the lowercase snake case of the collection or object name,
for example "Shop House" becomes web/raw/shop_house.glb. Objects and
materials Build scene tagged "maquette_name" (Blender renamed them on
import, "palette.001") export under the name their model file gives, so
names an app looks up ("lid", "screen") ship as the model has them. When
several of one model's materials want the same name (the bake gives each
baked object its own copy of a shared material) the first keeps it and the
others ship as "<name>_2", "<name>_3". The pipeline's own custom properties
("maquette_skip_bake" and the like) are removed, so no file carries them as
extras. Cameras and lights
are never exported. Geometry stays uncompressed (no Draco): the Compress
step applies meshopt and KTX2 from the presets.

Motion: a model's armature and skin export with it, and so does every
animation clip on its objects (their actions, active or in NLA tracks),
each as one glTF animation under its own name, sampled at the scene's
frame rate. Build scene has already removed the clips the Motion view
keeps off the site.

Baked light: --bake is the Bake light folder. For every object listed in
its bake.json the export
- keeps the object's UV sets: the texture UV as TEXCOORD_0 and "lightmap"
  as TEXCOORD_1;
- adds the object's <name>_light.png as the emissive texture of each of its
  materials that does not glow itself, sampled with the "lightmap" UV (an
  Image Texture fed by a UV Map node into the Principled BSDF's Emission
  Color, strength 1). A material with an emission of its own (a window's
  sky, a screen) keeps it and gets no lightmap;
- sets the material custom properties {"lightmap": <scale>, "lightmapUv":
  1} on those materials, written as glTF material extras (export_extras is
  on).
A web page that finds those extras moves the emissive texture to a light
map: albedo * lightmap * scale, the baked light at its real brightness
(three.js: lightMap = emissiveMap, lightMapIntensity = scale * PI, then
clear the emissive). Viewers that know nothing about it show the lightmap
as a glow. Objects without a bake (an asset set to skip the bake, a mesh
that glows, every object when bake.json says "baked": false) export as
they are, lit in real time, and a "lightmap" UV set left on them is
dropped, so they ship no second UV set. The blend is never saved, so these
edits only end up in the GLBs.
"""

import argparse
import json
import os
import re
import sys

import bpy

MODELS = "Models"
STUDIO = "Studio"
EXPORTED_TYPES = {"MESH", "EMPTY", "ARMATURE", "CURVE", "SURFACE", "META", "FONT"}
SOURCE_NAME = "maquette_name"
# The pipeline's own tags start with this; none of them ships.
TAG_PREFIX = "maquette_"


def log(message):
    print(f"[export_web] {message}", flush=True)


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="export_web.py")
    parser.add_argument("--bake", default="blender/bake", help="the Bake light folder")
    parser.add_argument("--out", required=True, help="folder for the GLB files")
    return parser.parse_args(argv)


def snake(name):
    text = re.sub(r"[^0-9a-zA-Z]+", "_", name).strip("_").lower()
    return text or "model"


def with_children(objects):
    out = []
    for obj in objects:
        out.append(obj)
        out.extend(obj.children_recursive)
    return [o for o in out if o.type in EXPORTED_TYPES]


def groups(scene):
    """(file name, objects) pairs, in a stable order."""
    found = []
    models = bpy.data.collections.get(MODELS)
    in_models = set()
    if models is not None:
        for child in sorted(models.children, key=lambda c: c.name):
            objects = [o for o in child.all_objects if o.type in EXPORTED_TYPES]
            in_models.update(child.all_objects)
            if objects:
                found.append((snake(child.name), objects))
    studio = bpy.data.collections.get(STUDIO)
    skip = set(studio.all_objects) if studio is not None else set()
    roots = [
        o
        for o in scene.objects
        if o.parent is None and o not in in_models and o not in skip and o.type in EXPORTED_TYPES
    ]
    for obj in sorted(roots, key=lambda o: o.name):
        found.append((snake(obj.name), with_children([obj])))
    return found


def load_bake(folder):
    """The baked objects from bake.json, {} when there is no bake."""
    path = os.path.join(folder, "bake.json")
    if not os.path.isfile(path):
        log(f"no {path}, materials export as they are")
        return {}
    with open(path, encoding="utf-8") as fh:
        report = json.load(fh)
    if not report.get("baked"):
        log("no baked light, materials export as they are")
        return {}
    return report.get("objects", {})


def principled(material):
    if material is None or material.node_tree is None:
        return None
    return next((n for n in material.node_tree.nodes if n.type == "BSDF_PRINCIPLED"), None)


def glows(material):
    """True for a material that lights itself, as Bake light decides it: a
    live emission on its Principled BSDF, or an unlit shader without one."""
    if material is None or material.node_tree is None:
        return False
    bsdf = principled(material)
    if bsdf is None:
        return any(n.type in {"EMISSION", "BACKGROUND"} for n in material.node_tree.nodes)
    strength = bsdf.inputs["Emission Strength"]
    color = bsdf.inputs["Emission Color"]
    strong = strength.is_linked or strength.default_value > 0.0
    lit = color.is_linked or max(color.default_value[:3]) > 0.0
    return strong and lit


def drop_lightmap_uv(obj):
    """Removes a "lightmap" UV set nothing samples, so it does not ship."""
    uvs = obj.data.uv_layers
    layer = uvs.get("lightmap")
    if layer is None or len(uvs) < 2:
        return False
    uvs.remove(layer)
    return True


def hook_lightmap(obj, entry, folder):
    """Puts the object's lightmap on its materials as the emissive texture,
    leaving every material that glows on its own as it is."""
    uvs = obj.data.uv_layers
    index = uvs.find(entry.get("uv", "lightmap"))
    path = os.path.join(folder, entry["file"])
    if index < 0 or not os.path.isfile(path):
        log(f"{obj.name}: no lightmap UV or no {path}, exported without baked light")
        return False
    image = bpy.data.images.load(os.path.abspath(path), check_existing=True)
    hooked = 0
    kept = 0
    for slot in obj.material_slots:
        material = slot.material
        bsdf = principled(material)
        if bsdf is None:
            continue
        if glows(material):
            kept += 1
            continue
        tree = material.node_tree
        uv = tree.nodes.new("ShaderNodeUVMap")
        uv.uv_map = uvs[index].name
        tex = tree.nodes.new("ShaderNodeTexImage")
        tex.image = image
        tree.links.new(uv.outputs["UV"], tex.inputs["Vector"])
        tree.links.new(tex.outputs["Color"], bsdf.inputs["Emission Color"])
        bsdf.inputs["Emission Strength"].default_value = 1.0
        material["lightmap"] = float(entry["scale"])
        material["lightmapUv"] = index
        hooked += 1
    glowing = f", {kept} keep their own glow" if kept else ""
    log(
        f"lightmap {entry['file']} on {obj.name}: {hooked} materials{glowing}, "
        f"scale {entry['scale']}, {len(uvs)} UV sets, lightmap is TEXCOORD_{index}"
    )
    return hooked > 0


def take_source_names():
    """The names Build scene kept from each model file, by datablock. The tags
    come off, so they do not export as extras."""
    found = {}
    for pool in (bpy.data.objects, bpy.data.materials):
        for item in pool:
            if SOURCE_NAME in item:
                found[item.as_pointer()] = str(item[SOURCE_NAME])
                del item[SOURCE_NAME]
    return found


def shipped_names(items, originals):
    """What each item ships as: its file's name, or its own when it has
    none. Blender keeps names unique, so when several items of one export
    want the same name (the bake gives every baked object its own copy of a
    shared material) the first keeps it and the others get "_2", "_3"."""
    taken = set()
    names = []
    for item in items:
        base = originals.get(item.as_pointer()) or item.name
        name, n = base, 2
        while name in taken:
            name, n = f"{base}_{n}", n + 1
        taken.add(name)
        names.append(name)
    return names


def own_names(objects, originals):
    """Gives these objects and their materials the names they ship with for
    one export. A datablock outside the export that holds one of those
    names steps aside under a name that never ships. Returns how to undo
    it."""
    materials = []
    for obj in objects:
        for slot in getattr(obj, "material_slots", []):
            if slot.material is not None and slot.material not in materials:
                materials.append(slot.material)
    undo = []
    for pool, items in ((bpy.data.objects, list(objects)), (bpy.data.materials, materials)):
        wanted = shipped_names(items, originals)
        moving = [(item, name) for item, name in zip(items, wanted) if item.name != name]
        # First every mover lets go of its name, so the names can swap.
        for i, (item, _) in enumerate(moving):
            undo.append((item, item.name))
            item.name = f"export_tmp_{i}"
        inside = set(items)
        for _, name in moving:
            holder = pool.get(name)
            if holder is not None and holder not in inside:
                undo.append((holder, holder.name))
                holder.name = f"{name}.held"
        for item, name in moving:
            item.name = name
    return undo


def drop_tags():
    """Removes the pipeline's own custom properties ("maquette_skip_bake"
    and the like) from every object and material, so they do not ship as
    glTF extras."""
    for pool in (bpy.data.objects, bpy.data.materials):
        for item in pool:
            for key in [k for k in item.keys() if k.startswith(TAG_PREFIX)]:
                del item[key]


def undo_names(undo):
    """Puts every name back. All of them let go first, so none collides."""
    first = {}
    for item, name in undo:
        first.setdefault(item.as_pointer(), (item, name))
    restore = list(first.values())
    for i, (item, _) in enumerate(restore):
        item.name = f"export_undo_{i}"
    for item, name in restore:
        item.name = name


def export(path, objects):
    bpy.ops.object.select_all(action="DESELECT")
    for obj in objects:
        obj.hide_set(False)
        obj.select_set(True)
    bpy.context.view_layer.objects.active = objects[0]
    bpy.ops.export_scene.gltf(
        filepath=os.path.abspath(path),
        export_format="GLB",
        use_selection=True,
        export_apply=True,
        export_extras=True,
        export_cameras=False,
        export_lights=False,
        export_yup=True,
        export_skins=True,
        export_animations=True,
        export_animation_mode="ACTIONS",
    )


def clip_count(path):
    """How many animations a written GLB holds, for the log."""
    with open(path, "rb") as fh:
        data = fh.read()
    length = int.from_bytes(data[12:16], "little")
    return len(json.loads(data[20 : 20 + length]).get("animations", []))


def main():
    args = parse_args()
    scene = bpy.context.scene
    baked = load_bake(args.bake)
    lit = set()
    for name, entry in baked.items():
        obj = bpy.data.objects.get(name)
        if obj is None or obj.type != "MESH":
            log(f"{name} is in bake.json but is no mesh in this blend, skipped")
            continue
        if hook_lightmap(obj, entry, args.bake):
            lit.add(obj.name)
    if baked:
        log(f"baked light on {len(lit)} of {len(baked)} objects")
    dropped = sum(
        drop_lightmap_uv(o) for o in scene.objects if o.type == "MESH" and o.name not in lit
    )
    if dropped:
        log(f"dropped the unused lightmap UV set of {dropped} meshes without baked light")
    found = groups(scene)
    if not found:
        log("nothing to export: add .glb files to models/ and run Build scene again")
        sys.exit(1)
    os.makedirs(args.out, exist_ok=True)
    # Files from an earlier export of a model that is gone would ship too.
    for old in os.listdir(args.out):
        if old.endswith(".glb"):
            os.remove(os.path.join(args.out, old))
    originals = take_source_names()
    drop_tags()
    names = set()
    for name, objects in found:
        unique, n = name, 2
        while unique in names:
            unique, n = f"{name}_{n}", n + 1
        names.add(unique)
        path = os.path.join(args.out, f"{unique}.glb")
        undo = own_names(objects, originals)
        try:
            export(path, objects)
        finally:
            undo_names(undo)
        clips = clip_count(path)
        moving = f", {clips} clip{'s' if clips != 1 else ''}" if clips else ""
        log(f"wrote {path} ({len(objects)} objects{moving}, {os.path.getsize(path) // 1024} KB)")
    log(f"exported {len(names)} models to {args.out}")


main()
