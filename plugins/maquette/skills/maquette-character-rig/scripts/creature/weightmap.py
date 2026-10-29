# weightmap.py: colour every vertex by its blended bone weights (chain hue, darker per segment),
# draw each bone as a coloured stick with a chain label, render front/side/top/back/persp.
# Optional 4th arg: a chain name to isolate (others grey). Usage: -- in.glb outprefix [chain]
import bpy, sys, math; sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(sys.argv[sys.argv.index("--")-1])))
from shlib import *
from groups import *
from mathutils import Vector
args = sys.argv[sys.argv.index("--")+1:]
glb, out = args[:2]; only = args[2] if len(args) > 2 else None
arm, me, stray = load(glb)
for s in stray: s.hide_render = True
m = me.data
gname = {g.index: g.name for g in me.vertex_groups}
attr = m.color_attributes.new("WCOL", "FLOAT_COLOR", "POINT")
for v in m.vertices:
    c = [0.0, 0.0, 0.0]; tot = 0
    for g in v.groups:
        b = gname[g.group]
        col = bone_color(b) if (not only or BONE2CHAIN.get(b) == only) else (0.82, 0.82, 0.82, 1)
        for k in range(3): c[k] += col[k] * g.weight
        tot += g.weight
    if tot == 0: c = [1, 0, 1]
    else: c = [x / tot for x in c]
    attr.data[v.index].color = (*c, 1)
m.color_attributes.active_color = attr
# bone sticks + labels
mw = arm.matrix_world
def stick(name, a, b, col, r=0.006):
    d = b - a; L = d.length
    if L < 1e-5: return
    bpy.ops.mesh.primitive_cone_add(vertices=6, radius1=r*1.6, radius2=r*0.4, depth=L, location=(a + b) / 2)
    o = bpy.context.object; o.name = name
    o.rotation_euler = d.to_track_quat("Z", "Y").to_euler()
    ca = o.data.color_attributes.new("WCOL", "FLOAT_COLOR", "POINT")
    for i in range(len(o.data.vertices)): ca.data[i].color = col
    o.show_in_front = True
for b in arm.data.bones:
    ch = BONE2CHAIN.get(b.name, "body")
    if only and ch != only: continue
    stick("STK_" + b.name, mw @ b.head_local, mw @ b.tail_local, (0.05, 0.05, 0.05, 1) if ch == "body" else bone_color(b.name))
labels = []
for ch, bs in CHAINS.items():
    if only and ch != only: continue
    b = arm.data.bones[bs[-1]]
    p = mw @ b.tail_local
    bpy.ops.object.text_add(location=p)
    t = bpy.context.object; t.data.body = ch.replace("leg_", "").replace("rear_", "r").replace("spike_", "s") if not only else bs[0]+"..."+bs[-1]
    t.data.size = 0.035; t.data.align_x = "CENTER"
    bpy.ops.object.convert(target="MESH"); t = bpy.context.object
    ca = t.data.color_attributes.new("WCOL", "FLOAT_COLOR", "POINT")
    for i in range(len(t.data.vertices)): ca.data[i].color = (1, 1, 1, 1)
    labels.append(t)
sc = bpy.context.scene; sc.display.shading.color_type = "VERTEX"
lo, hi = bounds(me); ctr = (lo + hi) / 2
for name, (yw, pt) in VIEWS.items():
    d = view_dir(yw, pt)   # labels face the camera
    rot = (-d).to_track_quat("-Z", "Y").to_euler()
    for t in labels: t.rotation_euler = rot; t.location = t.location  # keep
    for t in labels:
        base = t.get("base")
        if base is None: t["base"] = list(t.location); base = t["base"]
        t.location = Vector(base) + d * 0.15
    shot(f"{out}_{name}.png", ctr, 2.0, yw, pt, res=(900, 760), color="VERTEX", ortho=1.2)
