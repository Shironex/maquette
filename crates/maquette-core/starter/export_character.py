"""Export GLB for a character: the whole rig, meshes and clips in one file.

Maquette runs this headless, as the Export GLB step of the Character
showcase template:

    blender -b --factory-startup --python-exit-code 1 blender/character_built.blend \
        --python tools/export_character.py -- --out web/raw

The blend is the one the Build step (tools/build_character.py) writes.
Writes <out>/character.glb (change it with --name) with every armature, mesh
and action in the file: the skin and every clip are kept. Cameras and lights
stay out. Geometry stays
uncompressed: the Compress step applies the "character" preset, which keeps
meshopt on medium so skinned clips do not drift.
"""

import argparse
import os
import sys

import bpy


def log(message):
    print(f"[export_character] {message}", flush=True)


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="export_character.py")
    parser.add_argument("--out", required=True, help="folder for the GLB")
    parser.add_argument("--name", default="character", help="file name without .glb")
    return parser.parse_args(argv)


def main():
    args = parse_args()
    objects = [o for o in bpy.context.scene.objects if o.type in {"MESH", "ARMATURE", "EMPTY"}]
    if not any(o.type == "MESH" for o in objects):
        log("no meshes in this blend, nothing to export")
        sys.exit(1)
    bpy.ops.object.select_all(action="DESELECT")
    for obj in objects:
        obj.hide_set(False)
        obj.select_set(True)
    bpy.context.view_layer.objects.active = objects[0]
    os.makedirs(args.out, exist_ok=True)
    path = os.path.abspath(os.path.join(args.out, f"{args.name}.glb"))
    bpy.ops.export_scene.gltf(
        filepath=path,
        export_format="GLB",
        use_selection=True,
        export_animations=True,
        export_cameras=False,
        export_lights=False,
        export_yup=True,
    )
    clips = len(bpy.data.actions)
    log(f"wrote {path} ({len(objects)} objects, {clips} actions)")


main()
