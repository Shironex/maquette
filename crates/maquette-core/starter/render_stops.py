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
stops in order.

A scene without stops still gets one poster: a temporary camera is fitted to
the bounds of everything visible, at a three quarter view from above, and
renders overview.png. The camera is removed again and the blend is never
saved, so a new project has a poster before it has any cameras. Only a scene
with nothing to frame gets an empty list (the step still passes).

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


OVERVIEW = "overview"


def scene_corners(scene):
    """The world space bounding box corners of everything visible."""
    points = []
    for obj in scene.objects:
        if obj.type not in {"MESH", "CURVE", "SURFACE", "META", "FONT"} or obj.hide_render:
            continue
        points.extend(obj.matrix_world @ Vector(corner) for corner in obj.bound_box)
    return points


def overview_camera(scene):
    """A temporary camera framing the whole scene at a three quarter view, or None."""
    points = scene_corners(scene)
    if not points:
        return None
    centre = sum(points, Vector()) / len(points)
    data = bpy.data.cameras.new("maquette_overview")
    data.lens = 50.0
    camera = bpy.data.objects.new("maquette_overview", data)
    scene.collection.objects.link(camera)
    width = scene.render.resolution_x * scene.render.pixel_aspect_x
    height = scene.render.resolution_y * scene.render.pixel_aspect_y
    aspect = width / height if height else 1.0
    # The lens angle spans the longer side of the frame.
    wide = math.tan(data.angle / 2.0)
    tan_x, tan_y = (wide, wide / aspect) if aspect >= 1.0 else (wide * aspect, wide)
    azimuth, elevation = math.radians(45.0), math.radians(28.0)
    back = Vector((
        math.cos(elevation) * math.sin(azimuth),
        -math.cos(elevation) * math.cos(azimuth),
        math.sin(elevation),
    ))
    right = Vector((0.0, 0.0, 1.0)).cross(back).normalized()
    up = back.cross(right).normalized()
    # Back off along the view until every corner is inside the frame.
    distance = 0.1
    for point in points:
        rel = point - centre
        depth = rel.dot(back)
        distance = max(
            distance,
            depth + abs(rel.dot(right)) / tan_x,
            depth + abs(rel.dot(up)) / tan_y,
        )
    distance *= 1.06
    camera.location = centre + back * distance
    camera.rotation_euler = (-back).to_track_quat("-Z", "Y").to_euler()
    data.clip_end = max(data.clip_end, distance * 3.0)
    bpy.context.view_layer.update()
    return camera


def remove_camera(camera):
    data = camera.data
    bpy.data.objects.remove(camera, do_unlink=True)
    bpy.data.cameras.remove(data)


def render(scene, camera, args, name):
    path = os.path.abspath(os.path.join(args.out, f"{name}.png"))
    scene.camera = camera
    scene.render.filepath = path
    bpy.ops.render.render(write_still=True)
    log(f"rendered {camera.name} to {os.path.join(args.out, name + '.png')}")
    return {"camera": camera.name, "file": f"{name}.png", **view(camera, scene)}


def main():
    args = parse_args()
    os.makedirs(args.out, exist_ok=True)
    scene = bpy.context.scene
    cameras = stops(scene)
    written = []
    scene.render.engine = args.engine
    if args.engine == "CYCLES":
        scene.cycles.samples = args.samples
        scene.cycles.use_denoising = True
    scene.render.image_settings.file_format = "PNG"
    if cameras:
        for camera in cameras:
            written.append(render(scene, camera, args, snake(camera.name)))
    else:
        camera = overview_camera(scene)
        if camera is None:
            log("no camera stops and nothing visible to frame, skipping")
        else:
            log(
                f'no camera stops, so one framed three quarter poster of the whole scene; tag a camera with "{TAG}" '
                "or name it stop_... for your own"
            )
            try:
                entry = render(scene, camera, args, OVERVIEW)
                entry["camera"] = "Overview"
                entry["fallback"] = True
                written.append(entry)
            finally:
                remove_camera(camera)
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
