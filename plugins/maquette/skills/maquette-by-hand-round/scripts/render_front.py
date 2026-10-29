"""render_front.py: render a model from the front (and optionally other views) headless, files only.

Use it to look at a download that sits outside the Maquette workspace (render_thumbnail refuses
those). It never opens a window and never captures the screen.

Usage:
  blender -b --factory-startup -P render_front.py -- model.glb out_prefix [views=front] [size=512]
  views is a comma list of: front, side, back, q34. Writes <out_prefix>_<view>.png per view.

The model is framed by its bounds on a plain grey ground, textured, studio light, orthographic.
Blender space: Z up, the model faces -Y, so the front camera sits at -Y looking toward +Y.
"""
import math
import sys

import bpy
from mathutils import Vector

args = sys.argv[sys.argv.index("--") + 1:]
src, prefix = args[0], args[1]
views = (args[2] if len(args) > 2 else "front").split(",")
size = int(args[3]) if len(args) > 3 else 512
YAW = {"front": 0, "side": 90, "back": 180, "q34": 35}

bpy.ops.wm.read_factory_settings(use_empty=True)
ext = src.lower().rsplit(".", 1)[-1]
if ext in ("glb", "gltf"):
    bpy.ops.import_scene.gltf(filepath=src)
elif ext == "fbx":
    bpy.ops.import_scene.fbx(filepath=src)
elif ext == "obj":
    bpy.ops.wm.obj_import(filepath=src)
else:
    raise SystemExit("unsupported file type: " + ext)

meshes = [o for o in bpy.data.objects if o.type == "MESH"]
if not meshes:
    raise SystemExit("no mesh in " + src)
# The glTF importer adds bone display shapes: unskinned meshes next to a skinned one. Drop them.
if any(o.vertex_groups for o in meshes):
    keep = [o for o in meshes if o.vertex_groups]
    for o in [o for o in meshes if not o.vertex_groups]:
        bpy.data.objects.remove(o)
    meshes = keep
bpy.context.view_layer.update()
corners = [o.matrix_world @ Vector(c) for o in meshes for c in o.bound_box]
lo = Vector([min(c[i] for c in corners) for i in range(3)])
hi = Vector([max(c[i] for c in corners) for i in range(3)])
ctr, ext_ = (lo + hi) / 2, max((hi - lo).x, (hi - lo).y, (hi - lo).z)

sc = bpy.context.scene
sc.render.engine = "BLENDER_WORKBENCH"
sc.render.resolution_x = sc.render.resolution_y = size
sc.render.image_settings.file_format = "PNG"
sc.view_settings.view_transform = "Standard"
shading = sc.display.shading
shading.light = "STUDIO"
shading.color_type = "TEXTURE"
shading.background_type = "VIEWPORT"
shading.background_color = (0.55, 0.58, 0.62)
sc.world = sc.world or bpy.data.worlds.new("W")

cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
sc.collection.objects.link(cam)
cam.data.type = "ORTHO"
cam.data.ortho_scale = ext_ * 1.15
sc.camera = cam
for v in views:
    yaw = math.radians(YAW[v])
    d = Vector((math.sin(yaw), -math.cos(yaw), 0.0))
    cam.location = ctr + d * (ext_ * 3 + 2)
    cam.rotation_euler = (-d).to_track_quat("-Z", "Y").to_euler()
    sc.render.filepath = f"{prefix}_{v}.png"
    bpy.ops.render.render(write_still=True)
    print("RENDER", sc.render.filepath)
