"""Assembles one prop from parts made as separate models.

Each part is a GLB of its own (a frame, a door leaf, steps). Its meshes
join into one object named after the part, its origin moves to its pivot
(the bottom centre, the centre, a left or right hinge, or the origin it
was made with), and it goes under one empty named after the prop. A part
stays where its model put it, or goes where the plan says, turned about
the up axis. Run headless:

    blender -b --factory-startup --python assemble.py -- \
        --plan plan.json --output prop.glb --stats stats.json

plan.json:

    {"name": "sigil_gate", "parts": [
      {"file": "/abs/gate_door.glb", "name": "door_leaf", "pivot": "left-hinge",
       "at": [0.0, 0.0, 0.0], "turn": 0.0}]}

`at` is in glTF metres (Y up); null keeps the part where its model put it.
"""

import json
import math
import sys

import bpy
from mathutils import Matrix, Vector


def arg(name):
    argv = sys.argv[sys.argv.index("--") + 1 :]
    if name not in argv:
        raise SystemExit(f"assemble.py needs {name}")
    return argv[argv.index(name) + 1]


def to_blender(at):
    """glTF (Y up) to Blender (Z up)."""
    return Vector((at[0], -at[2], at[1]))


def to_gltf(v):
    return [round(v.x, 4) + 0.0, round(v.z, 4) + 0.0, round(-v.y, 4) + 0.0]


def import_part(path, name):
    """The part's meshes, transforms applied, as one object named `name`."""
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=path)
    added = [o for o in bpy.data.objects if o not in before]
    meshes = [o for o in added if o.type == "MESH"]
    if not meshes:
        raise SystemExit(f"{name}: the model has no mesh.")
    for obj in meshes:
        world = obj.matrix_world.copy()
        obj.parent = None
        obj.data = obj.data.copy()
        obj.data.transform(world)
        obj.matrix_world = Matrix.Identity(4)
    for obj in added:
        if obj not in meshes:
            bpy.data.objects.remove(obj, do_unlink=True)
    for obj in bpy.context.view_layer.objects:
        obj.select_set(obj in meshes)
    bpy.context.view_layer.objects.active = meshes[0]
    if len(meshes) > 1:
        bpy.ops.object.join()
    part = bpy.context.view_layer.objects.active
    part.name = name
    part.data.name = name
    return part


def pivot_of(obj, kind):
    points = [v.co for v in obj.data.vertices]
    low = Vector((min(p[i] for p in points) for i in range(3)))
    high = Vector((max(p[i] for p in points) for i in range(3)))
    mid = (low + high) / 2
    return {
        "origin": Vector((0.0, 0.0, 0.0)),
        "centre": mid,
        "bottom-centre": Vector((mid.x, mid.y, low.z)),
        "left-hinge": Vector((low.x, mid.y, low.z)),
        "right-hinge": Vector((high.x, mid.y, low.z)),
    }[kind]


def main():
    with open(arg("--plan"), encoding="utf-8") as f:
        plan = json.load(f)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    root = bpy.data.objects.new(plan["name"], None)
    bpy.context.scene.collection.objects.link(root)
    nodes = []
    for spec in plan["parts"]:
        part = import_part(spec["file"], spec["name"])
        pivot = pivot_of(part, spec.get("pivot", "origin"))
        part.data.transform(Matrix.Translation(-pivot))
        at = spec.get("at")
        part.location = pivot if at is None else to_blender(at)
        part.rotation_mode = "XYZ"
        part.rotation_euler = (0.0, 0.0, math.radians(spec.get("turn", 0.0)))
        part.parent = root
        nodes.append({"name": part.name, "at": to_gltf(part.location), "pivot": to_gltf(pivot)})
    bpy.ops.export_scene.gltf(filepath=arg("--output"), export_format="GLB")
    with open(arg("--stats"), "w", encoding="utf-8") as out:
        json.dump({"root": root.name, "parts": nodes}, out)


main()
