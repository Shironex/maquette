"""A blend as the bake leaves it, for the export name test: a desk holds the
real "palette", and the laptop's three baked parts each have their own copy
of it, all tagged with the name from the model file."""

import sys

import bpy

bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene
models = bpy.data.collections.new("Models")
scene.collection.children.link(models)


def model(name, parts):
    col = bpy.data.collections.new(name)
    models.children.link(col)
    for obj_name, mat_name, tag in parts:
        mesh = bpy.data.meshes.new(obj_name)
        mesh.from_pydata([(0, 0, 0), (1, 0, 0), (0, 1, 0)], [], [(0, 1, 2)])
        obj = bpy.data.objects.new(obj_name, mesh)
        col.objects.link(obj)
        mat = bpy.data.materials.get(mat_name) or bpy.data.materials.new(mat_name)
        if tag:
            mat["maquette_name"] = tag
        obj.data.materials.append(mat)
        obj["maquette_skip_bake"] = True


model("Desk", [("desk_top", "palette", None)])
model(
    "Laptop",
    [
        ("body", "palette.001", "palette"),
        ("lid", "palette.002", "palette"),
        ("screen", "screen", None),
        ("keys", "palette.003", "palette"),
    ],
)
bpy.ops.wm.save_as_mainfile(filepath=sys.argv[-1])
