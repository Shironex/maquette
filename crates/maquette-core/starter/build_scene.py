"""Build scene: put every model from the models folder into one Blender scene.

Maquette runs this headless, as the Build scene step:

    blender -b --factory-startup --python-exit-code 1 --python tools/build_scene.py -- \
        --base blender/scene.blend --models models --out blender/scene_built.blend

- When the base blend exists it is the starting point (your cameras, lights
  and hand placed objects). Otherwise the scene starts empty, with a sun and
  a grey world so the posters are lit.
- Each models/*.glb or *.gltf is imported into its own collection under
  "Models", named after the file. A model whose collection is already in the
  base blend is left as you placed it.
- Models that overlap at the origin are laid out side by side along X.
- The result is saved to --out. The base blend is never written.

Edit freely: this file is yours. Maquette only reruns the step when this
script, the base blend or a model changes.
"""

import argparse
import glob
import os
import sys

import bpy
from mathutils import Vector

MODELS = "Models"
STUDIO = "Studio"


def log(message):
    print(f"[build_scene] {message}", flush=True)


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="build_scene.py")
    parser.add_argument("--base", default="blender/scene.blend", help="optional starting blend")
    parser.add_argument("--models", default="models", help="folder with .glb or .gltf files")
    parser.add_argument("--out", required=True, help="where to save the built blend")
    return parser.parse_args(argv)


def start_scene(base):
    if os.path.isfile(base):
        log(f"starting from {base}")
        bpy.ops.wm.open_mainfile(filepath=os.path.abspath(base))
        return
    log(f"no {base} yet, starting from an empty scene")
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    studio = bpy.data.collections.new(STUDIO)
    scene.collection.children.link(studio)
    sun = bpy.data.objects.new("Sun", bpy.data.lights.new("Sun", type="SUN"))
    sun.data.energy = 3.0
    sun.rotation_euler = (0.9, 0.0, 0.8)
    studio.objects.link(sun)
    world = bpy.data.worlds.new("World")
    world.color = (0.18, 0.18, 0.18)
    scene.world = world


def models_collection(scene):
    found = bpy.data.collections.get(MODELS)
    if found is None:
        found = bpy.data.collections.new(MODELS)
    if found.name not in scene.collection.children:
        scene.collection.children.link(found)
    return found


def import_model(path, parent):
    """Imports one file into a new collection named after it."""
    name = os.path.splitext(os.path.basename(path))[0]
    if name in bpy.data.collections:
        log(f"{name} is already in the base scene, kept as placed")
        return None
    before = set(bpy.data.objects)
    try:
        # No icosphere bone shapes for rigged models: they would export as a mesh.
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
    log(f"imported {os.path.basename(path)}: {len(added)} objects")
    return target


def bounds(collection):
    """World space min and max corners of the meshes in a collection."""
    lo = Vector((float("inf"),) * 3)
    hi = Vector((float("-inf"),) * 3)
    for obj in collection.all_objects:
        if obj.type != "MESH":
            continue
        for corner in obj.bound_box:
            point = obj.matrix_world @ Vector(corner)
            lo = Vector(map(min, lo, point))
            hi = Vector(map(max, hi, point))
    if lo.x == float("inf"):
        return None
    return lo, hi


def overlaps(a, b):
    return all(a[0][i] < b[1][i] and b[0][i] < a[1][i] for i in range(3))


def lay_out(collections):
    """Moves the models into a row along X when any two of them overlap."""
    boxes = [(c, bounds(c)) for c in collections]
    boxes = [(c, b) for c, b in boxes if b is not None]
    if not any(overlaps(a[1], b[1]) for i, a in enumerate(boxes) for b in boxes[i + 1 :]):
        return
    gap = 0.25 * max((b[1] - b[0]).length for _, b in boxes)
    widths = [b[1].x - b[0].x for _, b in boxes]
    x = -(sum(widths) + gap * (len(boxes) - 1)) / 2
    for (collection, (lo, _hi)), width in zip(boxes, widths):
        shift = Vector((x - lo.x, 0.0, 0.0))
        for obj in collection.objects:
            if obj.parent is None:
                obj.location += shift
        x += width + gap
    bpy.context.view_layer.update()
    log(f"laid {len(boxes)} models out side by side")


def main():
    args = parse_args()
    start_scene(args.base)
    scene = bpy.context.scene
    parent = models_collection(scene)
    files = sorted(glob.glob(os.path.join(args.models, "*.glb")))
    files += sorted(glob.glob(os.path.join(args.models, "*.gltf")))
    if not files:
        log(f"no .glb or .gltf files in {args.models}, saving the scene without models")
    added = [c for c in (import_model(f, parent) for f in files) if c is not None]
    bpy.context.view_layer.update()
    if len(added) > 1:
        lay_out(added)
    out = os.path.abspath(args.out)
    os.makedirs(os.path.dirname(out), exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=out)
    log(f"saved {args.out} with {len(parent.children)} models")


main()
