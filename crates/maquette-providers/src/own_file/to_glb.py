"""Turns one model file (.gltf, .fbx or .obj) into a .glb, headless.

Usage: blender -b --python to_glb.py -- <model> <out.glb>
"""

import os
import sys

import bpy


def main():
    args = sys.argv[sys.argv.index("--") + 1 :]
    model, out = args[0], args[1]
    ext = os.path.splitext(model)[1].lower()
    bpy.ops.wm.read_factory_settings(use_empty=True)
    if ext in (".glb", ".gltf"):
        bpy.ops.import_scene.gltf(filepath=model)
    elif ext == ".fbx":
        bpy.ops.import_scene.fbx(filepath=model, use_anim=True, ignore_leaf_bones=False)
    elif ext == ".obj":
        # Its .mtl and textures sit beside it, where the importer looks.
        bpy.ops.wm.obj_import(filepath=model)
    else:
        raise SystemExit(f"Cannot read {model}: only .glb, .gltf, .fbx and .obj files.")
    if not any(o.type == "MESH" for o in bpy.context.scene.objects):
        raise SystemExit(f"{model} has no mesh in it.")
    bpy.ops.export_scene.gltf(filepath=out, export_format="GLB")
    print(f"[to_glb] wrote {out}")


main()
