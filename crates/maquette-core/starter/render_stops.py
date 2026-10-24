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

Each entry also carries the camera in glTF coordinates (Y up), so the
Inspector can fly to it: "position", "target" (along the view axis, at the
focus distance when depth of field is on, else as far as the world origin
is ahead, at least a metre) and "fov", the vertical field of view in degrees.
"""

import argparse
import json
import math
import os
import re
import sys

import bpy
from mathutils import Vector

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


def gltf_xyz(v):
    """Blender Z up to glTF Y up."""
    return [round(v.x, 4), round(v.z, 4), round(-v.y, 4)]


def view(camera, scene):
    """Position, target and vertical field of view of a camera, in glTF terms."""
    world = camera.matrix_world
    position = world.translation.copy()
    forward = (world.to_3x3() @ Vector((0.0, 0.0, -1.0))).normalized()
    data = camera.data
    depth = data.dof.focus_distance if data.dof.use_dof and data.dof.focus_distance > 0 else 0.0
    if depth <= 0:
        depth = max((Vector((0.0, 0.0, 0.0)) - position).dot(forward), 1.0)
    width = scene.render.resolution_x * scene.render.pixel_aspect_x
    height = scene.render.resolution_y * scene.render.pixel_aspect_y
    aspect = width / height if height else 1.0
    fit = data.sensor_fit
    if fit == "VERTICAL" or (fit == "AUTO" and aspect < 1.0):
        vfov = data.angle_y if fit == "VERTICAL" else data.angle
    else:
        hfov = data.angle_x if fit == "HORIZONTAL" else data.angle
        vfov = 2.0 * math.atan(math.tan(hfov / 2.0) / aspect)
    return {
        "position": gltf_xyz(position),
        "target": gltf_xyz(position + forward * depth),
        "fov": round(math.degrees(vfov), 3),
    }


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
            written.append({"camera": camera.name, "file": f"{name}.png", **view(camera, scene)})
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
