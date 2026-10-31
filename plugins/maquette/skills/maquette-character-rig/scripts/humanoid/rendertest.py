"""rendertest.py: headless pose test renders for a Mixamo-rigged GLB (Workbench, files only).

Usage:
  Blender -b --factory-startup -P rendertest.py -- --glb a.glb [--glb b.glb] --out sheet.png
          [--poses rest,raise,forward,hand,twist,head,stride] [--views front,side]
          [--clips walk.001:0,walk.001:0.5] [--colors colors.npy] [--labels before,after] [--cell 300x400]
          [--rows rows.json]
Poses are applied in armature (world) space: each rotation is about a world axis through the bone head,
so they read the same on any Mixamo rig regardless of bone roll. With several --glb, columns are
glb x view, rows are poses; the sheet is one PNG. --colors renders per-vertex debug colours instead of
the texture (an (V,3) float npy in glTF vertex order, which Blender's importer preserves).
Views and pose axes follow the way the model really faces, read from its shoulders, so "front" shows the
face whatever way the file points. --rows writes one JSON with each cell's file and, for every pose and
file, the worst edge stretch and seam gap in mm at a one metre model, and where the worst edge is.
"""
import bpy, sys, os, math
import numpy as np
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
cw, ch = map(int, opt("--cell", "300x400").split("x"))
frame = opt("--frame")
fx, fz, fs = map(float, (frame or "0,0.6,1.45").split(","))
fy = 0.0
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

def auto_frame(me):
    """Centre and window for a model of any size (a one unit tall model gets the old default)."""
    global fx, fy, fz, fs
    if frame:
        return
    co = [me.matrix_world @ v.co for v in me.data.vertices]
    lo = Vector([min(c[i] for c in co) for i in range(3)]); hi = Vector([max(c[i] for c in co) for i in range(3)])
    h = max(hi.z - lo.z, 1e-3)
    fx, fy = (lo.x + hi.x) / 2, (lo.y + hi.y) / 2
    fz, fs = lo.z + 0.6 * h, 1.45 * h

def edge_data(me):
    """Rest edges and vertex groups that share a position (UV seam copies)."""
    m = me.data
    co = np.zeros(len(m.vertices) * 3); m.vertices.foreach_get("co", co); co = co.reshape(-1, 3)
    ed = np.zeros(len(m.edges) * 2, int); m.edges.foreach_get("vertices", ed); ed = ed.reshape(-1, 2)
    _, inv = np.unique(np.round(co, 5), axis=0, return_inverse=True)
    inv = inv.ravel()
    order = np.argsort(inv)
    seams = [g for g in np.split(order, np.flatnonzero(np.diff(inv[order])) + 1) if len(g) > 1]
    return co, ed, seams

def measure(me, data, h):
    """Worst stretch and seam gap of the posed mesh against rest, in mm at a one metre tall model."""
    rest, ed, seams = data
    ev = me.evaluated_get(bpy.context.evaluated_depsgraph_get()).data
    co = np.zeros(len(ev.vertices) * 3); ev.vertices.foreach_get("co", co); co = co.reshape(-1, 3)
    d = np.abs(np.linalg.norm(co[ed[:, 0]] - co[ed[:, 1]], axis=1) - np.linalg.norm(rest[ed[:, 0]] - rest[ed[:, 1]], axis=1))
    k = int(d.argmax()) if len(d) else 0
    gap = max((float(np.linalg.norm(co[g] - co[g].mean(0), axis=1).max()) for g in seams), default=0.0)
    return dict(stretch_mm=round(float(d[k]) / h * 1000, 1), seam_gap_mm=round(gap / h * 1000, 1), at=[round(float(x), 3) for x in rest[ed[k, 0]]])

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
    sc.display.shading.background_color = (0.55, 0.58, 0.62)
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
    sc.collection.objects.link(cam)
    cam.data.type = "ORTHO"
    cam.data.ortho_scale = fs
    sc.camera = cam
    txt = bpy.data.curves.new("lbl", "FONT"); txt.size = 0.045
    tob = bpy.data.objects.new("lbl", txt); sc.collection.objects.link(tob)
    return cam, tob

def place(cam, tob, view, text, follow=None):
    ctr = Vector((fx, fy, fz)) + (Vector((follow.x, follow.y, 0)) if follow is not None else Vector())
    yaw = {"front": 0, "side": 90, "back": 180, "left": -90, "q34": 35, "bl": 135, "br": -135, "fl": 60}[view]
    y = math.radians(yaw + YAW0)
    d = Vector((math.sin(y), -math.cos(y), 0))  # camera offset direction; front = -Y
    cam.location = ctr + d * 5
    cam.rotation_euler = (-d).to_track_quat("-Z", "Y").to_euler()
    tob.data.body = text
    right = Vector((math.cos(y), math.sin(y), 0))
    tob.location = ctr + d * 4 - right * 0.36 * fs + Vector((0, 0, 0.43 * fs))
    tob.data.size = 0.031 * fs
    tob.rotation_euler = (math.radians(90), 0, y)

cells = {}
metrics = {}
cam = tob = None
for gi_, glb in enumerate(glbs):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=glb)
    for o in list(bpy.data.objects):
        if o.type == "MESH" and not o.vertex_groups:
            bpy.data.objects.remove(o)  # importer's bone display shape
    arm = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    me = next(o for o in bpy.data.objects if o.type == "MESH")
    set_facing(arm)
    if gi_ == 0:
        auto_frame(me)
    edges = edge_data(me) if rows_json else None
    height = max(me.dimensions.z, 1e-3)
    if colors:
        c = np.load(colors)
        attr = me.data.color_attributes.new("dbg", "FLOAT_COLOR", "POINT")
        rgba = np.concatenate([c, np.ones((len(c), 1))], 1).astype(np.float32)
        attr.data.foreach_set("color", rgba.ravel())
        me.data.color_attributes.active_color = attr
    cam, tob = setup_scene()
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
        if colors:
            sh = bpy.context.scene.display.shading
    if gi_ == 0:   # a clip the file does not hold under that name is left out, not a failed sheet
        clips = [c for c in clips if c.rsplit(":", 1)[0] in bpy.data.actions]
    rows = poses + ["clip:" + c for c in clips]
    for r in rows:
        reset(arm)
        if r.startswith("clip:"):
            n, frac = r[5:].rsplit(":", 1)
            set_clip(arm, n, float(frac))
        else:
            apply_pose(arm, r)
        if rows_json:
            metrics[f"{labels[gi_]}/{r}"] = measure(me, edges, height)
        for v in views:
            hp = (arm.matrix_world @ arm.pose.bones[P + "Hips"].head) if r.startswith("clip:") else None
            place(cam, tob, v, f"{labels[gi_]}  {r.replace('clip:', '')}  {v}", hp)
            fp = os.path.join(tmpdir, f"{gi_}_{r.replace(':', '_')}_{v}.png")
            bpy.context.scene.render.filepath = fp
            bpy.ops.render.render(write_still=True)
            cells[(gi_, r, v)] = fp

if rows_json:
    import json
    json.dump(dict(labels=labels, views=views, rows=poses + ["clip:" + c for c in clips],
                   cells={f"{labels[g]}/{r}/{v}": os.path.abspath(p) for (g, r, v), p in cells.items()}, metrics=metrics), open(rows_json, "w"), indent=1)
# assemble sheet: rows = poses, cols = glb x view
rows = poses + ["clip:" + c for c in clips]
cols = [(g, v) for g in range(len(glbs)) for v in views]
sheet = np.ones((ch * len(rows), cw * len(cols), 4), np.float32)
for ri, r in enumerate(rows):
    for ci, (g, v) in enumerate(cols):
        im = bpy.data.images.load(cells[(g, r, v)])
        px = np.array(im.pixels[:], np.float32).reshape(ch, cw, 4)[::-1]
        sheet[ri * ch:(ri + 1) * ch, ci * cw:(ci + 1) * cw] = px
        bpy.data.images.remove(im)
        if ci and cols[ci - 1][0] != g:
            sheet[ri * ch:(ri + 1) * ch, ci * cw:ci * cw + 3, :3] = 0.1
img = bpy.data.images.new("sheet", sheet.shape[1], sheet.shape[0], alpha=True)
img.pixels.foreach_set(sheet[::-1].ravel())
img.filepath_raw = out
img.file_format = "PNG"
img.save()
print("SHEET", out)
