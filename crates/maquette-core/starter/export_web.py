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

--bake is the Bake light folder. The starter export only logs what bake.json
says; when you bake for real, hook the maps up to the materials here.
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
        export_cameras=False,
        export_lights=False,
        export_yup=True,
    )


def main():
    args = parse_args()
    bake_report = os.path.join(args.bake, "bake.json")
    if os.path.isfile(bake_report):
        with open(bake_report, encoding="utf-8") as fh:
            baked = json.load(fh).get("baked", False)
        log("using baked light" if baked else "no baked light, materials export as they are")
    scene = bpy.context.scene
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
