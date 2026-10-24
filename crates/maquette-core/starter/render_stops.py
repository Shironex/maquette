"""Capture posters: one still per camera stop, for the site's camera tour.

Maquette runs this headless, as the Capture posters step:

    blender -b --factory-startup --python-exit-code 1 blender/scene_baked.blend \
        --python tools/render_stops.py -- --out web/posters

A camera is a stop when either is true:
- it has the custom property "maquette_stop" set to a true value (a number
  sets the order: 1, 2, 3), or
- its name starts with "stop_" or "Stop " (for example "stop_01_porch").
Each stop renders to <out>/<camera name in snake case>.png at the scene's
resolution, in Cycles by default. --out always gets stops.json listing the
stops in order; a scene without stops gets an empty list and the step still
passes, so a new project runs before it has any cameras.
"""

import argparse
import json
import os
import re
import sys

import bpy

TAG = "maquette_stop"


def log(message):
    print(f"[render_stops] {message}", flush=True)


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="render_stops.py")
    parser.add_argument("--out", required=True, help="folder for the posters and stops.json")
    parser.add_argument("--engine", default="CYCLES", help="CYCLES, or BLENDER_EEVEE to keep it fast")
    parser.add_argument("--samples", type=int, default=32, help="Cycles samples per poster")
    return parser.parse_args(argv)


def snake(name):
    text = re.sub(r"[^0-9a-zA-Z]+", "_", name).strip("_").lower()
    return text or "stop"


def stops(scene):
    """The tagged cameras, ordered by their tag number, then by name."""
    found = []
    for obj in scene.objects:
        if obj.type != "CAMERA":
            continue
        tag = obj.get(TAG)
        named = obj.name.startswith("stop_") or obj.name.startswith("Stop ")
        if tag or named:
            order = tag if isinstance(tag, (int, float)) and not isinstance(tag, bool) else 1e9
            found.append((order, obj.name, obj))
    return [obj for _, _, obj in sorted(found, key=lambda t: (t[0], t[1]))]


def main():
    args = parse_args()
    os.makedirs(args.out, exist_ok=True)
    scene = bpy.context.scene
    cameras = stops(scene)
    written = []
    if not cameras:
        log(f'no camera stops: tag a camera with the "{TAG}" property or name it stop_..., skipping')
    else:
        scene.render.engine = args.engine
        if args.engine == "CYCLES":
            scene.cycles.samples = args.samples
            scene.cycles.use_denoising = True
        scene.render.image_settings.file_format = "PNG"
        for camera in cameras:
            name = snake(camera.name)
            path = os.path.abspath(os.path.join(args.out, f"{name}.png"))
            scene.camera = camera
            scene.render.filepath = path
            bpy.ops.render.render(write_still=True)
            log(f"rendered {camera.name} to {os.path.join(args.out, name + '.png')}")
            written.append({"camera": camera.name, "file": f"{name}.png"})
    report = {
        "width": scene.render.resolution_x * scene.render.resolution_percentage // 100,
        "height": scene.render.resolution_y * scene.render.resolution_percentage // 100,
        "stops": written,
    }
    with open(os.path.join(args.out, "stops.json"), "w", encoding="utf-8") as fh:
        json.dump(report, fh, indent=2)
        fh.write("\n")
    log(f"wrote stops.json with {len(written)} stops")


main()
