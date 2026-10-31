# shlib.py: shared helpers for headless rig analysis of auto-rigged creature GLBs (non-humanoid).
# The per-creature tables live in rigconfig.py: the one in RIG_CONFIG_DIR if set, else the one beside this file.
# load(glb) imports a GLB into an empty scene and returns (armature, skinned mesh, stray unskinned meshes).
# shot(...) renders a Workbench image from an orbit camera aimed at the mesh bounds (no screen capture).
import bpy, math, os, sys
if os.environ.get("RIG_CONFIG_DIR"): sys.path.insert(0, os.environ["RIG_CONFIG_DIR"])
from rigconfig import *
import rigconfig as _rc
from mathutils import Vector
# Facing: FRONT in rigconfig.py is the world direction the creature looks along in Blender space
# (default -Y). LEFT is the creature's own left. Every view, pose axis and walk direction derives from these.
FRONT = Vector(getattr(_rc, "FRONT", (0, -1, 0))).normalized()
UP = Vector((0, 0, 1))
LEFT = UP.cross(FRONT).normalized()
def view_dir(yaw, pitch):
    """Unit vector from the target toward the camera: yaw 0 = in front of the face, 90 = the creature's left side."""
    y, p = math.radians(yaw), math.radians(pitch)
    return (FRONT * math.cos(y) + LEFT * math.sin(y)) * math.cos(p) + UP * math.sin(p)
def yaw_of(v):
    """Yaw (degrees, same convention as view_dir) of a horizontal offset v from the body centre."""
    return math.degrees(math.atan2(v.dot(LEFT), v.dot(FRONT)))
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
# The app's studio colour, so a tile sits in its frame without an edge; the outline keeps a dark model apart.
BACKGROUND = (0.0307, 0.0284, 0.0242)
OUTLINE = (0.42, 0.44, 0.48)
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
def shot(path, ctr, dist=2.0, yaw=0.0, pitch=0.0, res=(640, 560), color="TEXTURE", ortho=0.0, xray=False, text=None):
    """yaw 0 looks at the creature's front (FRONT in rigconfig.py), 90 at its left side."""
    sc = bpy.context.scene
    cam = bpy.data.objects.get("SHOTCAM")
    if not cam:
        cam = bpy.data.objects.new("SHOTCAM", bpy.data.cameras.new("SHOTCAM")); sc.collection.objects.link(cam)
    d = view_dir(yaw, pitch)
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
    sh.show_object_outline = True; sh.object_outline_color = OUTLINE
    sh.background_type = "VIEWPORT"; sh.background_color = BACKGROUND
    sc.view_settings.view_transform = "Standard"
    sc.render.resolution_x, sc.render.resolution_y = res; sc.render.resolution_percentage = 100
    sc.render.film_transparent = False
    sc.world = sc.world or bpy.data.worlds.new("W")
    sc.render.filepath = path; bpy.ops.render.render(write_still=True)
def text_strip(text, width, height=30):
    """A strip of text as (height, width, 4) pixels, rendered with everything but the text hidden."""
    import numpy as np
    sc = bpy.context.scene
    hidden = [o for o in bpy.data.objects if o.type == "MESH" and not o.hide_render]
    for o in hidden: o.hide_render = True
    res = (sc.render.resolution_x, sc.render.resolution_y)
    cam = bpy.data.objects.get("SHOTCAM")
    cam.location = (1000, -5, 0); cam.rotation_euler = (math.radians(90), 0, 0)
    cam.data.type = "ORTHO"; cam.data.ortho_scale = 1.0
    cu = bpy.data.curves.get("STRIP") or bpy.data.curves.new("STRIP", "FONT")
    t = bpy.data.objects.get("STRIP") or bpy.data.objects.new("STRIP", cu)
    if t.name not in sc.collection.objects: sc.collection.objects.link(t)
    cu.body = text; cu.size = 0.55 * height / width
    t.hide_render = False
    t.rotation_euler = (math.radians(90), 0, 0); t.location = (1000 - 0.48, 0, -0.2 * height / width)
    caption_was = bpy.data.objects.get("CAPTION")
    if caption_was: caption_was.hide_render = True
    sc.render.resolution_x, sc.render.resolution_y = width, height
    sc.render.filepath = os.path.join(os.path.dirname(sc.render.filepath), "_strip.png")
    bpy.ops.render.render(write_still=True)
    im = bpy.data.images.load(sc.render.filepath)
    out = np.array(im.pixels[:], dtype=np.float32).reshape(height, width, 4)
    bpy.data.images.remove(im); os.remove(sc.render.filepath)
    t.hide_render = True
    if caption_was: caption_was.hide_render = False
    for o in hidden: o.hide_render = False
    sc.render.resolution_x, sc.render.resolution_y = res
    return out
def montage(rows, path, gap=4, bg=0.12, labels=None):
    """Paste PNG tiles into one sheet: rows is a list of lists of PNG paths (None leaves a blank cell).
    labels maps a tile's path to the text written across its top, in the sheet only."""
    import numpy as np
    imgs = {}
    for r in rows:
        for p in r:
            if p and p not in imgs:
                im = bpy.data.images.load(os.path.abspath(p)); w, h = im.size
                imgs[p] = np.array(im.pixels[:], dtype=np.float32).reshape(h, w, 4)
                bpy.data.images.remove(im)
                if labels and p in labels:
                    strip = text_strip(labels[p], w)
                    imgs[p][h - strip.shape[0]:] = strip   # Blender pixels start at the bottom
    cw = max(a.shape[1] for a in imgs.values()); chh = max(a.shape[0] for a in imgs.values())
    ncol = max(len(r) for r in rows); W = ncol * cw + (ncol + 1) * gap; H = len(rows) * chh + (len(rows) + 1) * gap
    out = np.full((H, W, 4), bg, dtype=np.float32); out[..., 3] = 1
    for ri, r in enumerate(rows):          # Blender pixels start at the bottom row: row 0 goes on top
        y0 = H - (ri + 1) * (chh + gap)
        for ci, p in enumerate(r):
            if not p: continue
            a = imgs[p]; x0 = gap + ci * (cw + gap)
            out[y0:y0 + a.shape[0], x0:x0 + a.shape[1]] = a
    im = bpy.data.images.new("SHEET", W, H, alpha=True); im.pixels[:] = out.ravel()
    im.filepath_raw = os.path.abspath(path); im.file_format = "PNG"; im.save(); bpy.data.images.remove(im)
