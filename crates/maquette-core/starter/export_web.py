"""Export GLB: one binary glTF per model, ready for the Compress step.

Maquette runs this headless, as the Export GLB step:

    blender -b --factory-startup --python-exit-code 1 blender/scene_baked.blend \
        --python tools/export_web.py -- --bake blender/bake --out web/raw

What becomes a file:
- each child collection of "Models" (one per imported model) is one GLB;
- each other top level mesh, empty or armature outside "Models" and
  "Studio" is one GLB with its children.
File names are the lowercase snake case of the collection or object name,
for example "Shop House" becomes web/raw/shop_house.glb. Cameras and lights
are never exported. Geometry stays uncompressed (no Draco): the Compress
step applies meshopt and KTX2 from the presets.

Baked light: --bake is the Bake light folder. For every object listed in
its bake.json the export
- keeps the object's UV sets: the texture UV as TEXCOORD_0 and "lightmap"
  as TEXCOORD_1;
- adds the object's <name>_light.png as the emissive texture, sampled with
  the "lightmap" UV (an Image Texture fed by a UV Map node into the
  Principled BSDF's Emission Color, strength 1);
- sets the material custom properties {"lightmap": <scale>, "lightmapUv":
  1}, written as glTF material extras (export_extras is on).
A web page that finds those extras swaps the material for an unlit one
that shows albedo * emissive texture * lightmap, the baked light at its
real brightness. Viewers that know nothing about it show the lightmap as a
glow. Objects without a bake, and every object when bake.json says
"baked": false, export as they are and are lit in real time. The blend is
never saved, so these material edits only end up in the GLBs.
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


def hook_lightmap(obj, entry, folder):
    """Puts the object's lightmap on its materials as the emissive texture."""
    uvs = obj.data.uv_layers
    index = uvs.find(entry.get("uv", "lightmap"))
    path = os.path.join(folder, entry["file"])
    if index < 0 or not os.path.isfile(path):
        log(f"{obj.name}: no lightmap UV or no {path}, exported without baked light")
        return False
    image = bpy.data.images.load(os.path.abspath(path), check_existing=True)
    hooked = 0
    for slot in obj.material_slots:
        material = slot.material
        bsdf = principled(material)
        if bsdf is None:
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
    log(
        f"lightmap {entry['file']} on {obj.name}: {hooked} materials, "
        f"scale {entry['scale']}, {len(uvs)} UV sets, lightmap is TEXCOORD_{index}"
    )
    return hooked > 0


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
    )


def main():
    args = parse_args()
    scene = bpy.context.scene
    baked = load_bake(args.bake)
    lit = 0
    for name, entry in baked.items():
        obj = bpy.data.objects.get(name)
        if obj is None or obj.type != "MESH":
            log(f"{name} is in bake.json but is no mesh in this blend, skipped")
            continue
        lit += hook_lightmap(obj, entry, args.bake)
    if baked:
        log(f"baked light on {lit} of {len(baked)} objects")
    found = groups(scene)
    if not found:
        log("nothing to export: add .glb files to models/ and run Build scene again")
        sys.exit(1)
    os.makedirs(args.out, exist_ok=True)
    # Files from an earlier export of a model that is gone would ship too.
    for old in os.listdir(args.out):
        if old.endswith(".glb"):
            os.remove(os.path.join(args.out, old))
    names = set()
    for name, objects in found:
        unique, n = name, 2
        while unique in names:
            unique, n = f"{name}_{n}", n + 1
        names.add(unique)
        path = os.path.join(args.out, f"{unique}.glb")
        export(path, objects)
        log(f"wrote {path} ({len(objects)} objects, {os.path.getsize(path) // 1024} KB)")
    log(f"exported {len(names)} models to {args.out}")


main()
