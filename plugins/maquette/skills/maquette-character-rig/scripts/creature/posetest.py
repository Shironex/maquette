# posetest.py: bend each chain in turn (world-axis rotations about each bone head, so it works on any
# rig without knowing bone rolls), render rest/posed tiles with weight colours, and measure per welded
# piece the non-rigid deviation: Kabsch best rigid fit of the piece's posed verts to its rest verts, max
# residual in model units (model is ~1.0 wide, so 0.01 = 1% of body width). Rigid crystal should be ~0.
# Pieces longer than ART_EXT span a joint (whole leg crystals) and are reported apart as "articulated".
# Usage: -- in.glb outdir tag
import bpy, sys, math, json; sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(sys.argv[sys.argv.index("--")-1])))
from shlib import *
from groups import *
from mathutils import Vector, Matrix
glb, outd, tag = sys.argv[sys.argv.index("--")+1:]
arm, me, stray = load(glb)
for s in stray: s.hide_render = True
m = me.data; gname = {g.index: g.name for g in me.vertex_groups}
attr = m.color_attributes.new("WCOL", "FLOAT_COLOR", "POINT"); m.color_attributes.active_color = attr
for v in m.vertices:
    c = [0, 0, 0]; t = 0
    for g in v.groups:
        col = bone_color(gname[g.group])
        for k in range(3): c[k] += col[k] * g.weight
        t += g.weight
    attr.data[v.index].color = (*[x / max(t, 1e-6) for x in c], 1)
# welded pieces
N = len(m.vertices); par = list(range(N))
def f(a):
    while par[a] != a: par[a] = par[par[a]]; a = par[a]
    return a
for e in m.edges:
    a, b = f(e.vertices[0]), f(e.vertices[1])
    if a != b: par[a] = b
key = {}
for v in m.vertices:
    k = tuple(round(c, 5) for c in v.co)
    if k in key:
        a, b = f(v.index), f(key[k])
        if a != b: par[a] = b
    else: key[k] = v.index
piece = [f(i) for i in range(N)]
pid = {r: i for i, r in enumerate(sorted(set(piece)))}
piece = [pid[r] for r in piece]
edges = [tuple(e.vertices) for e in m.edges]
rest = [v.co.copy() for v in m.vertices]
L0 = [(rest[a] - rest[b]).length for a, b in edges]
import posing; posing.bind(arm)
from posing import rot_world, chain_dir, pose_chain, reset
import numpy as np
R0 = np.array([tuple(me.matrix_world @ v) for v in rest])
PI = np.array(piece)
PIDX = {p: np.where(PI == p)[0] for p in set(piece)}
EXT = {p: float((R0[ix].max(0) - R0[ix].min(0)).max()) for p, ix in PIDX.items()}
ART_EXT = 0.25
def kabsch_err(A, B):
    ca, cb = A.mean(0), B.mean(0); H = (A - ca).T @ (B - cb)
    U, S, Vt = np.linalg.svd(H); d = np.sign(np.linalg.det(Vt.T @ U.T))
    Rm = Vt.T @ np.diag([1, 1, d]) @ U.T
    res = np.linalg.norm((A - ca) @ Rm.T + cb - B, axis=1)
    return float(res.max()), int(res.argmax())
def measure():
    dg = bpy.context.evaluated_depsgraph_get(); ev = me.evaluated_get(dg).data
    co = np.array([tuple(me.matrix_world @ v.co) for v in ev.vertices])
    worst = {}
    for p, ix in PIDX.items():
        if len(ix) < 3: continue
        e, k = kabsch_err(R0[ix], co[ix]); worst[p] = (e, int(ix[k]))
    return worst, co
lo, hi = bounds(me); ctr = (lo + hi) / 2
psize = {}
for p in piece: psize[p] = psize.get(p, 0) + 1
report = {}
tiles = []
for ch in CHAINS:
    reset(); pose_chain(ch)
    worst, co = measure()
    rig = sorted(((e, p) for p, (e, k) in worst.items() if e > 0.004 and EXT[p] <= ART_EXT), reverse=True)
    art = sorted(((e, p) for p, (e, k) in worst.items() if e > 0.004 and EXT[p] > ART_EXT), reverse=True)
    report[ch] = dict(rigid=[dict(piece=p, n=psize[p], ext=round(EXT[p], 3), err=round(e, 4), at=[round(x, 3) for x in co[worst[p][1]]]) for e, p in rig],
                      articulated=[dict(piece=p, n=psize[p], ext=round(EXT[p], 3), err=round(e, 4)) for e, p in art])
    a, b = chain_dir(CHAINS[ch]); c = (a + b) / 2
    if ch.startswith("leg") or ch.startswith("rear"): c = Vector((c.x, c.y, 0.3))
    yaw = math.degrees(math.atan2(c.x, -c.y)) if (c.xy.length > 0.05) else 30
    if ch.startswith("spike") or ch == "body": yaw += 60
    fnp = f"{outd}/{tag}_{ch}_pose.png"; fnr = f"{outd}/{tag}_{ch}_rest.png"
    worst_txt = f"rigid>4mm: {len(rig)}  max {rig[0][0]*1000:.0f}mm" if rig else "rigid pieces ok"
    shot(fnp, c, 2.0, yaw, 25, res=(360, 300), color="VERTEX", ortho=0.8, text=f"{ch} POSED  {worst_txt}")
    reset()
    shot(fnr, c, 2.0, yaw, 25, res=(360, 300), color="VERTEX", ortho=0.8, text=f"{ch} rest")
    tiles += [fnr, fnp]
    print("POSE", ch, "rigid>4mm:", len(rig), [(r["n"], r["err"], r["at"]) for r in report[ch]["rigid"][:4]], "| art:", [(r["n"], r["err"]) for r in report[ch]["articulated"][:3]])
reset()
json.dump(report, open(f"{outd}/{tag}_report.json", "w"), indent=1)
open(f"{outd}/{tag}_tiles.txt", "w").write(" ".join(tiles))
