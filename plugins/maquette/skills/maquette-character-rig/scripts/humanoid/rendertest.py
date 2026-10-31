"""rendertest.py: headless pose test renders for a Mixamo-rigged GLB (Workbench, files only).

Usage:
  Blender -b --factory-startup -P rendertest.py -- --glb a.glb [--glb b.glb] --out sheet.png
          [--poses rest,raise,forward,hand,twist,head,stride] [--views front,side]
          [--clips walk.001:0,walk.001:0.5] [--colors colors.npy] [--labels before,after] [--cell 480x640]
          [--rows rows.json]
Poses are applied in armature (world) space: each rotation is about a world axis through the bone head,
so they read the same on any Mixamo rig regardless of bone roll. With several --glb, columns are
glb x view, rows are poses; the sheet is one PNG. --colors renders per-vertex debug colours instead of
the texture (an (V,3) float npy in glTF vertex order, which Blender's importer preserves).
Views and pose axes follow the way the model really faces, read from its shoulders, so "front" shows the
face whatever way the file points. --rows writes one JSON with each cell's file and, for every pose and
file, the numbers of surface.py (stretch, stretch beyond the body, seam gap) and where the worst edge is.
Every row keeps the whole body in frame: one camera per row and view, fitted to that pose in all the files,
so a row's files compare exactly and a pose only zooms out when it does not fit the rest pose's window.
Cells carry no text, for a caption beside them; the sheet writes its labels in a strip over each cell.
"""
import bpy, sys, os, math
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np
import surface
from mathutils import Vector, Matrix, Quaternion

argv = sys.argv[sys.argv.index("--") + 1:]
def opt(name, default=None, many=False):
    vals = [argv[i + 1] for i, a in enumerate(argv) if a == name]
    return vals if many else (vals[0] if vals else default)
glbs = opt("--glb", many=True)
out = os.path.abspath(opt("--out"))   # Blender resolves a relative save path against its own folder
rows_json = opt("--rows")
poses = opt("--poses", "rest,raise,forward,hand,twist,head,stride").split(",")
views = opt("--views", "front,side").split(",")
clips = [c for c in opt("--clips", "").split(",") if c]
colors = opt("--colors")
labels = opt("--labels", ",".join(os.path.basename(g)[:-4] for g in glbs)).split(",")
cw, ch = map(int, opt("--cell", "480x640").split("x"))
frame = opt("--frame")   # x,z,scale: one fixed window instead of the fit per row
LABEL_H = 34             # the sheet's label strip over each cell, in pixels
# The app's studio colour, so a cell sits in its frame without an edge. The outline keeps a dark model apart.
BACKGROUND = (0.0307, 0.0284, 0.0242)
OUTLINE = (0.42, 0.44, 0.48)
# one cell folder per sheet, so runs in parallel never share files
tmpdir = os.path.abspath(out) + "_cells"
os.makedirs(tmpdir, exist_ok=True)

P = "mixamorig:"
UP = Vector((0, 0, 1))
LEFT, FRONT = Vector((1, 0, 0)), Vector((0, -1, 0))   # read per model by set_facing
YAW0 = 0.0

def set_facing(arm):
    """The character's left is where its left shoulder is; it faces LEFT x UP."""
    global LEFT, FRONT, YAW0
    bones = arm.data.bones
    a, b = bones.get(P + "LeftShoulder") or bones.get(P + "LeftArm"), bones.get(P + "RightShoulder") or bones.get(P + "RightArm")
    if a is None or b is None:
        return
    d = arm.matrix_world.to_3x3() @ (a.head_local - b.head_local)
    d.z = 0
    if d.length < 1e-5:
        return
    LEFT = d.normalized()
    FRONT = LEFT.cross(UP)
    YAW0 = math.degrees(math.atan2(FRONT.x, -FRONT.y))

VIEW_YAW = {"front": 0, "side": 90, "back": 180, "left": -90, "q34": 35, "bl": 135, "br": -135, "fl": 60}

def view_axes(view):
    """The way the camera sits from the model, and the way right runs across its picture."""
    y = math.radians(VIEW_YAW[view] + YAW0)
    return Vector((math.sin(y), -math.cos(y), 0)), Vector((math.cos(y), math.sin(y), 0))

def world_points(me):
    ev = me.evaluated_get(bpy.context.evaluated_depsgraph_get()).data
    co = np.zeros(len(ev.vertices) * 3); ev.vertices.foreach_get("co", co)
    return co.reshape(-1, 3) @ np.array(me.matrix_world.to_3x3()).T + np.array(me.matrix_world.translation)

def grow_bounds(found, key, points):
    """Adds a pose's points to what this row and view must show: (across low, high, up low, high)."""
    _, right = view_axes(key[1])
    across = points @ np.array(right)
    now = (float(across.min()), float(across.max()), float(points[:, 2].min()), float(points[:, 2].max()))
    was = found.get(key, now)
    found[key] = (min(was[0], now[0]), max(was[1], now[1]), min(was[2], now[2]), max(was[3], now[3]))

def window(bounds):
    """How tall a window shows these bounds with a little air, in a cell of this shape."""
    return max(bounds[3] - bounds[2], (bounds[1] - bounds[0]) * ch / cw, 1e-3) * 1.08

def row_scale(found, row):
    """One zoom for a whole row, never closer than the rest pose so standing poses stay the same size."""
    ours = [window(b) for (r, _), b in found.items() if r == row]
    rest = [window(b) for (r, _), b in found.items() if r == "rest"]
    return max(ours + rest)

def rot(axis, deg):
    return Quaternion(Vector(axis).normalized(), math.radians(deg)).to_matrix().to_4x4()

def world_rotate(arm, name, R):
    pb = arm.pose.bones.get(P + name)
    if pb is None:
        return
    bpy.context.view_layer.update()
    h = pb.head.copy()
    pb.matrix = Matrix.Translation(h) @ R @ Matrix.Translation(-h) @ pb.matrix
    bpy.context.view_layer.update()

def bone_dir(arm, name):
    b = arm.data.bones[P + name]
    return (b.tail_local - b.head_local).normalized()

def apply_pose(arm, pose):
    # Blender space: Z up, character faces -Y, character's left is +X
    down = Vector((0, 0, -1)); fwd = FRONT
    if pose == "raise":  # both arms out and up 70 degrees from where they hang
        for side, out_ in (("Left", LEFT), ("Right", -LEFT)):
            world_rotate(arm, side + "Arm", rot(down.cross(out_), 70))
    elif pose == "forward":
        for side in ("Left", "Right"):
            world_rotate(arm, side + "Arm", rot(down.cross(fwd), 75))
            world_rotate(arm, side + "ForeArm", rot(down.cross(fwd), 25))
    elif pose == "hand":  # wrist twist 90 about the forearm axis plus a 45 degree bend
        for side, s in (("Left", 1), ("Right", -1)):
            world_rotate(arm, side + "Hand", rot(bone_dir(arm, side + "ForeArm"), 90 * s))
            world_rotate(arm, side + "Hand", rot(fwd.cross(bone_dir(arm, side + "ForeArm")), 45))
            world_rotate(arm, side + "ForeArm", rot(bone_dir(arm, side + "ForeArm"), 30 * s))
    elif pose == "twist":
        for n in ("Spine", "Spine1", "Spine2"):
            world_rotate(arm, n, rot((0, 0, 1), 15))
    elif pose == "head":
        world_rotate(arm, "Neck", rot((0, 0, 1), 20))
        world_rotate(arm, "Head", rot((0, 0, 1), 40))
        world_rotate(arm, "Head", rot(LEFT, 15))
    elif pose == "stride":
        world_rotate(arm, "LeftUpLeg", rot(LEFT, -35))
        world_rotate(arm, "LeftLeg", rot(LEFT, 40))
        world_rotate(arm, "RightUpLeg", rot(LEFT, 25))
    elif pose == "rest":
        pass
    else:
        raise SystemExit("unknown pose " + pose)

def reset(arm):
    for pb in arm.pose.bones:
        pb.matrix_basis = Matrix.Identity(4)
    if arm.animation_data:
        arm.animation_data.action = None
    bpy.context.view_layer.update()

def set_clip(arm, name, frac):
    act = bpy.data.actions[name]
    ad = arm.animation_data or arm.animation_data_create()
    for t in list(ad.nla_tracks):
        t.mute = True
    ad.action = act
    try:
        if act.slots:
            ad.action_slot = act.slots[0]
    except AttributeError:
        pass
    f0, f1 = act.frame_range
    f = f0 + (f1 - f0) * frac
    bpy.context.scene.frame_set(int(math.floor(f)), subframe=f - math.floor(f))
    bpy.context.view_layer.update()

def setup_scene():
    sc = bpy.context.scene
    sc.render.engine = "BLENDER_WORKBENCH"
    sh = sc.display.shading
    sh.light = "STUDIO"
    sh.color_type = "VERTEX" if colors else "TEXTURE"
    sh.show_xray = "--xray" in argv
    sh.xray_alpha = 0.5
    sh.show_backface_culling = False
    sc.render.resolution_x, sc.render.resolution_y = cw, ch
    sc.render.film_transparent = False
    sc.world = sc.world or bpy.data.worlds.new("W")
    sc.render.image_settings.file_format = "PNG"
    sc.view_settings.view_transform = "Standard"
    sc.display.shading.background_type = "VIEWPORT"
    sc.display.shading.background_color = BACKGROUND
    sh.show_object_outline = True
    sh.object_outline_color = OUTLINE
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
    sc.collection.objects.link(cam)
    cam.data.type = "ORTHO"
    sc.camera = cam
    return cam

def place(cam, view, bounds, scale):
    """Aims the camera at the middle of what this row and view must show."""
    back, right = view_axes(view)
    middle = right * ((bounds[0] + bounds[1]) / 2) + UP * ((bounds[2] + bounds[3]) / 2)
    away = max(5.0, 3 * scale)
    cam.data.ortho_scale = scale
    cam.data.clip_end = away * 3
    cam.location = middle + back * away
    cam.rotation_euler = (-back).to_track_quat("-Z", "Y").to_euler()

def label_strip(cam, text, path):
    """A strip of the cell's width with its label, for the saved sheet."""
    sc = bpy.context.scene
    txt = bpy.data.curves.get("lbl") or bpy.data.curves.new("lbl", "FONT")
    tob = bpy.data.objects.get("lbl") or bpy.data.objects.new("lbl", txt)
    if tob.name not in sc.collection.objects:
        sc.collection.objects.link(tob)
    txt.body = text
    txt.size = 0.55 * LABEL_H / cw
    tob.rotation_euler = (math.radians(90), 0, 0)
    tob.location = (1000 - 0.47, 0, -0.2 * LABEL_H / cw)
    cam.data.ortho_scale = 1.0
    cam.location = (1000, -5, 0)
    cam.rotation_euler = (math.radians(90), 0, 0)
    sc.render.resolution_x, sc.render.resolution_y = cw, LABEL_H
    sc.render.filepath = path
    bpy.ops.render.render(write_still=True)
    sc.render.resolution_x, sc.render.resolution_y = cw, ch

def load(glb):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=glb)
    for o in list(bpy.data.objects):
        if o.type == "MESH" and not o.vertex_groups:
            bpy.data.objects.remove(o)  # importer's bone display shape
    arm = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    me = next(o for o in bpy.data.objects if o.type == "MESH")
    set_facing(arm)
    return arm, me

def pose_row(arm, row):
    reset(arm)
    if row.startswith("clip:"):
        n, frac = row[5:].rsplit(":", 1)
        set_clip(arm, n, float(frac))
    else:
        apply_pose(arm, row)

metrics = {}
found = {}
rows = None
# The first pass measures every pose in every file, so the cameras can fit them all.
for gi_, glb in enumerate(glbs):
    arm, me = load(glb)
    if gi_ == 0:   # a clip the file does not hold under that name is left out, not a failed sheet
        clips = [c for c in clips if c.rsplit(":", 1)[0] in bpy.data.actions]
        rows = poses + ["clip:" + c for c in clips]
    body = surface.Surface(arm, me) if rows_json else None
    for r in rows:
        pose_row(arm, r)
        if body:
            metrics[f"{labels[gi_]}/{r}"] = body.measure()
        points = world_points(me)
        for v in views:
            grow_bounds(found, (r, v), points)

if frame:
    fx, fz, fs = map(float, frame.split(","))
    for key in found:
        back, right = view_axes(key[1])
        across = right.x * fx
        found[key] = (across - fs / 2, across + fs / 2, fz - fs / 2, fz + fs / 2)

cells = {}
strips = {}
loaded = len(glbs) == 1   # one file is still in the scene from the first pass
for gi_, glb in enumerate(glbs):
    if not loaded:
        arm, me = load(glb)
    if colors:
        c = np.load(colors)
        attr = me.data.color_attributes.new("dbg", "FLOAT_COLOR", "POINT")
        rgba = np.concatenate([c, np.ones((len(c), 1))], 1).astype(np.float32)
        attr.data.foreach_set("color", rgba.ravel())
        me.data.color_attributes.active_color = attr
    cam = setup_scene()
    if "--bones" in argv:
        mat = bpy.data.materials.new("bonecol"); mat.diffuse_color = (0.0, 1.0, 0.2, 1)
        for b in arm.data.bones:
            if "Hand" in b.name and not b.name.endswith("Hand"):
                continue
            L = b.length
            bpy.ops.mesh.primitive_cylinder_add(vertices=6, radius=0.004, depth=L)
            c = bpy.context.active_object
            c.data.materials.append(mat)
            c.show_in_front = True
            c.parent = arm; c.parent_type = "BONE"; c.parent_bone = b.name
            # bone-parented child sits at the bone tail; local Y runs along the bone
            c.matrix_parent_inverse = Matrix.Identity(4)
            c.location = (0, -L / 2, 0)
            c.rotation_euler = (math.radians(90), 0, 0)
    for r in rows:
        pose_row(arm, r)
        scale = fs if frame else row_scale(found, r)
        for v in views:
            place(cam, v, found[(r, v)], scale)
            fp = os.path.join(tmpdir, f"{gi_}_{r.replace(':', '_')}_{v}.png")
            bpy.context.scene.render.filepath = fp
            bpy.ops.render.render(write_still=True)
            cells[(gi_, r, v)] = fp
    # The sheet's labels, in strips of their own so no cell carries text.
    for o in bpy.data.objects:
        if o.type == "MESH":
            o.hide_render = True
    for r in rows:
        for v in views:
            fp = os.path.join(tmpdir, f"label_{gi_}_{r.replace(':', '_')}_{v}.png")
            label_strip(cam, f"{labels[gi_]}  {r.replace('clip:', '')}  {v}", fp)
            strips[(gi_, r, v)] = fp

if rows_json:
    import json
    json.dump(dict(labels=labels, views=views, rows=poses + ["clip:" + c for c in clips],
                   cells={f"{labels[g]}/{r}/{v}": os.path.abspath(p) for (g, r, v), p in cells.items()}, metrics=metrics), open(rows_json, "w"), indent=1)
# assemble sheet: rows = poses, cols = glb x view, a label strip over each cell
cols = [(g, v) for g in range(len(glbs)) for v in views]
step = ch + LABEL_H
sheet = np.ones((step * len(rows), cw * len(cols), 4), np.float32)

def paste(path, y, x, w, h):
    im = bpy.data.images.load(path)
    sheet[y:y + h, x:x + w] = np.array(im.pixels[:], np.float32).reshape(h, w, 4)[::-1]
    bpy.data.images.remove(im)

for ri, r in enumerate(rows):
    for ci, (g, v) in enumerate(cols):
        paste(strips[(g, r, v)], ri * step, ci * cw, cw, LABEL_H)
        paste(cells[(g, r, v)], ri * step + LABEL_H, ci * cw, cw, ch)
        if ci and cols[ci - 1][0] != g:
            sheet[ri * step:(ri + 1) * step, ci * cw:ci * cw + 3, :3] = 0.1
img = bpy.data.images.new("sheet", sheet.shape[1], sheet.shape[0], alpha=True)
img.pixels.foreach_set(sheet[::-1].ravel())
img.filepath_raw = out
img.file_format = "PNG"
img.save()
print("SHEET", out)
