# shlib.py: shared helpers for headless rig analysis of auto-rigged creature GLBs (non-humanoid).
# The per-creature tables live in rigconfig.py: the one in RIG_CONFIG_DIR if set, else the one beside this file.
# load(glb) imports a GLB into an empty scene and returns (armature, skinned mesh, stray unskinned meshes).
# shot(...) renders a Workbench image from an orbit camera aimed at the mesh bounds (no screen capture).
import bpy, math, os, sys
if os.environ.get("RIG_CONFIG_DIR"): sys.path.insert(0, os.environ["RIG_CONFIG_DIR"])
from rigconfig import *
from mathutils import Vector
def load(glb):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.scene.render.fps = 30   # the glTF importer converts clip seconds to frames at scene fps
    bpy.ops.import_scene.gltf(filepath=glb)
    arm = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    me = next(o for o in bpy.data.objects if o.type == "MESH" and o.vertex_groups)
    stray = [o for o in bpy.data.objects if o.type == "MESH" and not o.vertex_groups]
    return arm, me, stray
def bounds(me):
    dg = bpy.context.evaluated_depsgraph_get(); ev = me.evaluated_get(dg)
    co = [me.matrix_world @ v.co for v in ev.data.vertices]
    lo = Vector([min(c[i] for c in co) for i in range(3)]); hi = Vector([max(c[i] for c in co) for i in range(3)])
    return lo, hi
VIEWS = {"front": (0, 0), "side": (90, 0), "back": (180, 0), "top": (0, 89), "persp": (35, 25)}
def caption(cam, text, ortho, res):
    """Text parented to the camera, top-left of frame (works headless, no ffmpeg drawtext needed)."""
    t = bpy.data.objects.get("CAPTION")
    if not t:
        cu = bpy.data.curves.new("CAPTION", "FONT"); t = bpy.data.objects.new("CAPTION", cu)
        bpy.context.scene.collection.objects.link(t)
        mat = bpy.data.materials.new("CAPMAT"); mat.diffuse_color = (1, 1, 0.6, 1); cu.materials.append(mat)
    t.parent = cam; t.data.body = text or ""
    asp = res[1] / res[0]
    half = (ortho / 2) if ortho else 0.36 * 1.0
    t.data.size = half * 0.09
    t.location = (-half * 0.96, half * asp * 0.96 - half * 0.09, -1.0 if not ortho else -0.5)
    t.rotation_euler = (0, 0, 0)
def shot(path, ctr, dist=2.0, yaw=0.0, pitch=0.0, res=(640, 560), color="TEXTURE", ortho=0.0, xray=False, front_axis=-1, text=None):
    """yaw 0 looks at the creature's front (front_axis=-1 means the creature faces -Y)."""
    sc = bpy.context.scene
    cam = bpy.data.objects.get("SHOTCAM")
    if not cam:
        cam = bpy.data.objects.new("SHOTCAM", bpy.data.cameras.new("SHOTCAM")); sc.collection.objects.link(cam)
    y, p = math.radians(yaw), math.radians(pitch)
    d = Vector((math.sin(y) * math.cos(p), front_axis * math.cos(y) * math.cos(p), math.sin(p)))
    cam.location = ctr + d * dist
    cam.rotation_euler = (ctr - cam.location).to_track_quat("-Z", "Y").to_euler()
    if ortho:
        cam.data.type = "ORTHO"; cam.data.ortho_scale = ortho
    else:
        cam.data.type = "PERSP"; cam.data.lens = 50
    sc.camera = cam
    caption(cam, text, ortho, res)
    sc.render.engine = "BLENDER_WORKBENCH"
    sh = sc.display.shading; sh.light = "STUDIO"; sh.color_type = color; sh.show_xray = xray
    sh.show_object_outline = False
    sc.render.resolution_x, sc.render.resolution_y = res; sc.render.resolution_percentage = 100
    sc.render.film_transparent = False
    sc.world = sc.world or bpy.data.worlds.new("W")
    sc.render.filepath = path; bpy.ops.render.render(write_still=True)
