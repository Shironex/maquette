"""Bakes the texture of the model a split came from onto its parts.

A split on a provider can come back without the texture (Meshy always
gives the parts flat vertex colours). This puts the texture back: every
part gets its own UV map and image, and Cycles bakes the base colour of
the source onto it, selected to active, by casting rays a short way out
from the part and back (a few percent of the model's size). The parts
keep their names, their places and their shapes; only their materials
change. Run headless:

    blender -b --factory-startup --python bake_back.py -- \
        --source source.glb --target split.glb --output baked.glb \
        --stats stats.json [--size 1024]
"""

import json
import os
import sys

import bpy
from mathutils import Vector


def arg(name, default=None):
    argv = sys.argv[sys.argv.index("--") + 1 :]
    if name in argv:
        return argv[argv.index(name) + 1]
    if default is None:
        raise SystemExit(f"bake_back.py needs {name}")
    return default


def import_glb(path):
    """Imports a GLB and returns the objects it added."""
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=path)
    return [o for o in bpy.data.objects if o not in before]


def select_only(objects, active):
    for o in bpy.context.view_layer.objects:
        o.select_set(False)
    for o in objects:
        o.select_set(True)
    bpy.context.view_layer.objects.active = active


def joined(meshes):
    """The source's meshes as one object, so every ray finds it."""
    select_only(meshes, meshes[0])
    if len(meshes) > 1:
        bpy.ops.object.join()
    return bpy.context.view_layer.objects.active


def diagonal(objects):
    points = [o.matrix_world @ Vector(c) for o in objects for c in o.bound_box]
    low = Vector((min(p[i] for p in points) for i in range(3)))
    high = Vector((max(p[i] for p in points) for i in range(3)))
    return max((high - low).length, 1e-4)


def unwrap(obj):
    """A fresh UV map, the only one, for the baked image."""
    mesh = obj.data
    while mesh.uv_layers:
        mesh.uv_layers.remove(mesh.uv_layers[0])
    mesh.uv_layers.new(name="baked")
    select_only([obj], obj)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.smart_project(island_margin=0.02)
    bpy.ops.object.mode_set(mode="OBJECT")
    for attr in list(mesh.color_attributes):
        mesh.color_attributes.remove(attr)


def baked_material(obj, size):
    """One material per part with an empty image node, active for the bake."""
    image = bpy.data.images.new(f"{obj.name}_baked", size, size, alpha=False)
    mat = bpy.data.materials.new(f"{obj.name}_baked")
    mat.use_nodes = True
    node = mat.node_tree.nodes.new("ShaderNodeTexImage")
    node.image = image
    mat.node_tree.nodes.active = node
    obj.data.materials.clear()
    obj.data.materials.append(mat)
    return mat, node, image


def mean_colour(image):
    """The mean colour of the texels the bake filled (the rest stay black)."""
    px = image.pixels[:]
    filled = [px[i : i + 3] for i in range(0, len(px), 4) if sum(px[i : i + 3]) > 0.01]
    if not filled:
        return [0.0, 0.0, 0.0]
    return [round(sum(c[k] for c in filled) / len(filled), 3) for k in range(3)]


def main():
    source_path = arg("--source")
    target_path = arg("--target")
    output = arg("--output")
    stats_path = arg("--stats")
    size = int(arg("--size", "1024"))
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = 1
    source_objects = import_glb(source_path)
    source_meshes = [o for o in source_objects if o.type == "MESH"]
    if not source_meshes:
        raise SystemExit("The source model has no mesh to bake from.")
    source = joined(source_meshes)
    parts = [o for o in import_glb(target_path) if o.type == "MESH"]
    if not parts:
        raise SystemExit("The split model has no parts to bake onto.")
    reach = diagonal([source])
    folder = os.path.dirname(os.path.abspath(output))
    baked = []
    for part in parts:
        unwrap(part)
        mat, node, image = baked_material(part, size)
        select_only([source, part], part)
        bpy.ops.object.bake(
            type="DIFFUSE",
            pass_filter={"COLOR"},
            use_selected_to_active=True,
            cage_extrusion=reach * 0.02,
            max_ray_distance=reach * 0.1,
            margin=4,
        )
        baked.append({"name": part.name, "mean": mean_colour(image)})
        image.filepath_raw = os.path.join(folder, f"{part.name}_baked.png")
        image.file_format = "PNG"
        image.save()
        image.pack()
        bsdf = mat.node_tree.nodes.get("Principled BSDF")
        mat.node_tree.links.new(node.outputs["Color"], bsdf.inputs["Base Color"])
    for obj in [o for o in bpy.data.objects if o not in parts and o.type != "EMPTY"]:
        bpy.data.objects.remove(obj, do_unlink=True)
    for obj in [o for o in bpy.data.objects if o.type == "EMPTY" and not o.children]:
        bpy.data.objects.remove(obj, do_unlink=True)
    bpy.ops.export_scene.gltf(filepath=output, export_format="GLB")
    with open(stats_path, "w", encoding="utf-8") as out:
        json.dump({"parts": baked, "size": size}, out)


main()
