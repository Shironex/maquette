"""Edge padding fixture, two modes:

    -- make <out.glb>   a square whose UV island is the left half of a 64 px
                        texture: dark red inside the island, white gaps
    -- read <in.glb>    prints the texture's mean colour in the island and
                        in the gap as JSON, after a "PAD:" marker
"""

import json
import sys

import bpy
import numpy as np

mode, path = sys.argv[sys.argv.index("--") + 1 :][:2]
bpy.ops.wm.read_factory_settings(use_empty=True)
SIZE = 64

if mode == "make":
    mesh = bpy.data.meshes.new("card")
    mesh.from_pydata([(0, 0, 0), (1, 0, 0), (1, 1, 0), (0, 1, 0)], [], [(0, 1, 2, 3)])
    uv = mesh.uv_layers.new(name="UVMap")
    for loop, coord in zip(uv.data, [(0.05, 0.05), (0.45, 0.05), (0.45, 0.95), (0.05, 0.95)]):
        loop.uv = coord
    obj = bpy.data.objects.new("card", mesh)
    bpy.context.scene.collection.objects.link(obj)
    image = bpy.data.images.new("atlas", SIZE, SIZE, alpha=True)
    px = np.ones((SIZE, SIZE, 4), np.float32)
    px[:, : SIZE // 2, :3] = (0.5, 0.05, 0.05)
    image.pixels.foreach_set(px.ravel())
    image.pack()
    material = bpy.data.materials.new("card")
    material.use_nodes = True
    tex = material.node_tree.nodes.new("ShaderNodeTexImage")
    tex.image = image
    bsdf = material.node_tree.nodes["Principled BSDF"]
    material.node_tree.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    mesh.materials.append(material)
    bpy.ops.export_scene.gltf(filepath=path, export_format="GLB")
else:
    bpy.ops.import_scene.gltf(filepath=path)
    image = next(i for i in bpy.data.images if i.size[0] > 0)
    w, h = image.size
    px = np.empty(w * h * image.channels, np.float32)
    image.pixels.foreach_get(px)
    px = px.reshape(h, w, image.channels)[..., :3]
    island = px[h // 4 : 3 * h // 4, w // 8 : 3 * w // 8].mean(axis=(0, 1))
    gap = px[:, 5 * w // 8 :].mean(axis=(0, 1))
    print("PAD:" + json.dumps({"size": w, "island": island.tolist(), "gap": gap.tolist()}))
