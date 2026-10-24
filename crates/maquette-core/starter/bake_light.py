"""Bake light: a starter stub that bakes nothing yet.

Maquette runs this headless, as the Bake light step:

    blender -b --factory-startup --python-exit-code 1 blender/scene_built.blend \
        --python tools/bake_light.py -- --out-dir blender/bake --out-blend blender/scene_baked.blend

A new project ships with real-time lighting only, so this step just passes
the built scene on: it saves it as --out-blend and writes bake.json in
--out-dir saying that nothing was baked. The export step reads that folder,
so the pipeline runs end to end from day one.

To bake for real, replace the body of bake() with your own: for example
give each mesh a lightmap UV, bake Cycles "COMBINED" or "DIFFUSE" into an
image per object, save the images into --out-dir as <object>_light.png and
list them in bake.json. Keep the two outputs (the blend and bake.json).
"""

import argparse
import json
import os
import sys

import bpy


def log(message):
    print(f"[bake_light] {message}", flush=True)


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="bake_light.py")
    parser.add_argument("--out-dir", required=True, help="folder for baked maps and bake.json")
    parser.add_argument("--out-blend", required=True, help="where to save the baked scene")
    return parser.parse_args(argv)


def bake(out_dir):
    """Returns the maps it wrote. The stub writes none."""
    meshes = [o.name for o in bpy.context.scene.objects if o.type == "MESH"]
    log(f"stub: nothing baked, {len(meshes)} meshes keep real-time lighting")
    return []


def main():
    args = parse_args()
    os.makedirs(args.out_dir, exist_ok=True)
    maps = bake(args.out_dir)
    report = {
        "baked": bool(maps),
        "maps": maps,
        "note": "Starter stub: no light was baked. Edit tools/bake_light.py to bake.",
    }
    with open(os.path.join(args.out_dir, "bake.json"), "w", encoding="utf-8") as fh:
        json.dump(report, fh, indent=2)
        fh.write("\n")
    out = os.path.abspath(args.out_blend)
    os.makedirs(os.path.dirname(out), exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=out, copy=True)
    log(f"saved {args.out_blend} and {os.path.join(args.out_dir, 'bake.json')}")


main()
