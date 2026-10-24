"""Build character: put the newest character model into a lit, framed scene.

Maquette runs this headless, as the Build step of the Character showcase
template:

    blender -b --factory-startup --python-exit-code 1 --python tools/build_character.py -- \
        --base blender/character.blend --models models --out blender/character_built.blend

- When the base blend exists it is the starting point (your lights, cameras
  and stage). Otherwise the scene starts empty with a grey world.
- The character is the newest GLB in --models: the highest "_v<n>" version
  (an accepted inbox model such as models/hero_v3.glb), then the newest
  file. Pick one by hand with --model models/hero_v2.glb. It is imported
  with its armature, skin and clips into a "Character" collection. A
  character whose collection is already in the base blend is kept as
  placed, so the base can hold a hand tuned copy.
- Every clip gets a fake user, so the ones not playing are saved too.
- When the scene has no light, a key light and a soft fill go into a
  "Studio" collection. When it has no camera stop, one camera named
  stop_01_front (with the "maquette_stop" property) frames the character
  from the front, so Capture posters has something to render.
- The result is saved to --out. The base blend is never written.

Edit freely: this file is yours. Maquette only reruns the step when this
script, the base blend or a model changes.
"""

import argparse
import glob
import math
import os
import re
import sys

import bpy
from mathutils import Vector

CHARACTER = "Character"
STUDIO = "Studio"
TAG = "maquette_stop"
VERSION = re.compile(r"_v(\d+)$")


def log(message):
    print(f"[build_character] {message}", flush=True)


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="build_character.py")
    parser.add_argument("--base", default="blender/character.blend", help="optional starting blend")
    parser.add_argument("--models", default="models", help="folder with the character .glb")
    parser.add_argument("--model", default="", help="this .glb instead of the newest one")
    parser.add_argument("--out", required=True, help="where to save the built blend")
    return parser.parse_args(argv)


def newest_model(folder):
    """The highest _v<n> GLB in the folder, then the most recently changed."""

    def key(path):
        found = VERSION.search(os.path.splitext(os.path.basename(path))[0])
        return (int(found.group(1)) if found else -1, os.path.getmtime(path), path)

    files = glob.glob(os.path.join(folder, "*.glb"))
    return max(files, key=key) if files else None


def start_scene(base):
    if os.path.isfile(base):
        log(f"starting from {base}")
        bpy.ops.wm.open_mainfile(filepath=os.path.abspath(base))
        return
    log(f"no {base} yet, starting from an empty scene")
    bpy.ops.wm.read_factory_settings(use_empty=True)
    world = bpy.data.worlds.new("World")
    world.color = (0.18, 0.18, 0.18)
    bpy.context.scene.world = world


def collection(scene, name):
    found = bpy.data.collections.get(name)
    if found is None:
        found = bpy.data.collections.new(name)
    if found.name not in scene.collection.children:
        scene.collection.children.link(found)
    return found


def import_character(path, parent):
    """Imports the GLB into a collection named after it, under "Character"."""
    name = os.path.splitext(os.path.basename(path))[0]
    if name in bpy.data.collections:
        log(f"{name} is already in the base scene, kept as placed")
        return bpy.data.collections[name]
    before = set(bpy.data.objects)
    try:
        # No icosphere bone shapes: they would export as a mesh.
        bpy.ops.import_scene.gltf(filepath=os.path.abspath(path), disable_bone_shape=True)
    except TypeError:  # an importer without the option
        bpy.ops.import_scene.gltf(filepath=os.path.abspath(path))
    added = [o for o in bpy.data.objects if o not in before]
    target = bpy.data.collections.new(name)
    parent.children.link(target)
    for obj in added:
        for owner in list(obj.users_collection):
            owner.objects.unlink(obj)
        target.objects.link(obj)
    rigs = sum(o.type == "ARMATURE" for o in added)
    meshes = sum(o.type == "MESH" for o in added)
    log(f"imported {os.path.basename(path)}: {meshes} meshes, {rigs} armatures")
    return target


def keep_clips():
    for action in bpy.data.actions:
        action.use_fake_user = True
    log(f"{len(bpy.data.actions)} clips: {', '.join(a.name for a in bpy.data.actions) or 'none'}")


def bounds(objects):
    """World space min and max corners of the meshes, or None."""
    lo = Vector((math.inf,) * 3)
    hi = Vector((-math.inf,) * 3)
    for obj in objects:
        if obj.type != "MESH":
            continue
        for corner in obj.bound_box:
            point = obj.matrix_world @ Vector(corner)
            lo = Vector(map(min, lo, point))
            hi = Vector(map(max, hi, point))
    return None if lo.x == math.inf else (lo, hi)


def add_lights(scene):
    if any(o.type == "LIGHT" for o in scene.objects):
        return
    studio = collection(scene, STUDIO)
    for name, energy, rotation in (
        ("Key", 3.0, (math.radians(50), 0.0, math.radians(-35))),
        ("Fill", 0.8, (math.radians(65), 0.0, math.radians(140))),
    ):
        light = bpy.data.objects.new(name, bpy.data.lights.new(name, type="SUN"))
        light.data.energy = energy
        light.data.angle = math.radians(8)
        light.rotation_euler = rotation
        studio.objects.link(light)
    log("no lights in the scene, added a key and a fill light")


def is_stop(obj):
    return obj.type == "CAMERA" and (
        bool(obj.get(TAG)) or obj.name.startswith("stop_") or obj.name.startswith("Stop ")
    )


def add_stop(scene, box):
    """A front camera that fits the character's height, 16:9."""
    if any(is_stop(o) for o in scene.objects):
        return
    lo, hi = box
    center = (lo + hi) / 2
    size = hi - lo
    data = bpy.data.cameras.new("stop_01_front")
    data.lens = 50
    camera = bpy.data.objects.new("stop_01_front", data)
    camera[TAG] = 1
    render = scene.render
    aspect = render.resolution_x / render.resolution_y
    half_v = math.atan(data.sensor_width / 2 / data.lens / aspect)
    half_h = math.atan(data.sensor_width / 2 / data.lens)
    # Far enough that the height (plus a margin) and the width fit.
    fit = max(size.z * 0.6 / math.tan(half_v), size.x * 0.6 / math.tan(half_h))
    distance = fit + size.y / 2
    # glTF models face +Z, which is -Y in Blender: look at the front.
    camera.location = center + Vector((0.0, -distance, size.z * 0.05))
    camera.rotation_euler = (center - camera.location).to_track_quat("-Z", "Y").to_euler()
    data.clip_end = max(100.0, distance * 4)
    collection(scene, STUDIO).objects.link(camera)
    scene.camera = camera
    log(f"no camera stop, added stop_01_front {distance:.2f} m in front of the character")


def main():
    args = parse_args()
    start_scene(args.base)
    scene = bpy.context.scene
    model = args.model or newest_model(args.models)
    parent = collection(scene, CHARACTER)
    if model:
        import_character(model, parent)
    else:
        log(f"no .glb in {args.models}: accept a character in the inbox, then run Build again")
    keep_clips()
    bpy.context.view_layer.update()
    box = bounds(parent.all_objects) or bounds(scene.objects)
    add_lights(scene)
    if box is not None:
        add_stop(scene, box)
    out = os.path.abspath(args.out)
    os.makedirs(os.path.dirname(out), exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=out)
    log(f"saved {args.out}")


main()
