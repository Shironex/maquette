# posetest.py: bend each chain in turn (world-axis rotations about each bone head, so it works on any
# rig without knowing bone rolls), render rest/posed tiles with weight colours, and measure per welded
# piece the non-rigid deviation: Kabsch best rigid fit of the piece's posed verts to its rest verts, max
# residual in model units (model is ~1.0 wide, so 0.01 = 1% of body width). Rigid crystal should be ~0.
# Rigid means what autoconfig.py and fix.py mean: smaller than RIGID_EXT, on a model that has pieces at all. Other pieces
# span a joint (whole leg crystals) and are reported apart as "articulated"; so is every island of one continuous shell
# (PIECE_RULES False), where edge islands are only UV islands and nothing has to hold rigid.
# Two surface checks catch tearing on meshes that are one continuous shell (no rigid pieces):
#   seam gap: vertices that share a position at rest (glTF splits them at UV seams) must stay together;
#             a gap means the two copies carry different weights and the surface cracks open there.
#   stretch:  the worst edge length ratio posed/rest (1.0 = unchanged), 99th percentile and max.
# The report's stretch_mm, beyond_body_mm and seam_gap_mm come from ../humanoid/surface.py, in mm at a model
# one metre along its longest side; beyond_body_mm is what the rows are judged by. The tiles carry no text:
# the sheet writes its labels over them, so the app can caption a tile itself.
# WELD_PIECES = False in rigconfig.py keeps edge islands as the pieces (use it when welding by shared
# positions merges the whole model into one piece, which pieces.py shows as WELDED_PIECES 1).
# Usage: -- in.glb outdir tag
import bpy, sys, math, json; sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(sys.argv[sys.argv.index("--")-1])))
from shlib import *
from groups import *
from mathutils import Vector, Matrix
import rigconfig as _rc
import os
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(sys.argv[sys.argv.index("--")-1])), "..", "humanoid"))
import surface as _surface
WELD = getattr(_rc, "WELD_PIECES", True)
PIECE_RULES = getattr(_rc, "PIECE_RULES", True)
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
key = {}; coloc = {}
for v in m.vertices:
    k = tuple(round(c, 5) for c in v.co)
    if k in key:
        coloc.setdefault(key[k], [key[k]]).append(v.index)
        if WELD:
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
SEAM = [np.array(g) for g in coloc.values()]
EA = np.array(edges); L0n = np.linalg.norm(R0[EA[:, 0]] - R0[EA[:, 1]], axis=1); EOK = L0n > 1e-6
def surface(co):
    gap = max((float(np.linalg.norm(co[g] - co[g].mean(0), axis=1).max()) for g in SEAM), default=0.0)
    ln = np.linalg.norm(co[EA[:, 0]] - co[EA[:, 1]], axis=1)[EOK]
    r = ln / L0n[EOK]
    at = R0[EA[EOK][int(r.argmax()), 0]]   # rest position of the worst edge, to find it on the model
    return gap, float(np.percentile(r, 99)), float(r.max()), [round(float(x), 3) for x in at], float(np.abs(ln - L0n[EOK]).max())
lo, hi = bounds(me); ctr = (lo + hi) / 2
BODY = _surface.Surface(arm, me)
# One window for every tile, wide enough for the whole body from any side with a chain bent out of it.
WIDE = (hi - lo).length * 1.2; FAR = max(2.0, WIDE * 2)
psize = {}
for p in piece: psize[p] = psize.get(p, 0) + 1
report = {}
tiles = []
labels = {}
for ch in CHAINS:
    reset(); pose_chain(ch)
    worst, co = measure()
    gap, s99, smax, sat, sabs = surface(co)
    held = BODY.measure(BODY.size)
    bent = [(e, p) for p, (e, k) in worst.items() if e > 0.004 * BODY.size]
    rig = sorted(((e, p) for e, p in bent if PIECE_RULES and EXT[p] < RIGID_EXT), reverse=True)
    art = sorted(((e, p) for e, p in bent if not (PIECE_RULES and EXT[p] < RIGID_EXT)), reverse=True)
    report[ch] = dict(rigid=[dict(piece=p, n=psize[p], ext=round(EXT[p], 3), err=round(e, 4), bend_mm=round(e / BODY.size * 1000, 1), at=[round(x, 3) for x in co[worst[p][1]]]) for e, p in rig],
                      articulated=[dict(piece=p, n=psize[p], ext=round(EXT[p], 3), err=round(e, 4)) for e, p in art],
                      seam_gap=round(gap, 4), stretch_p99=round(s99, 3), stretch_max=round(smax, 3), stretch_at=sat, stretch_mm=held["stretch_mm"],
                      beyond_body_mm=held["beyond_body_mm"], seam_gap_mm=held["seam_gap_mm"])
    a, b = chain_dir(CHAINS[ch]); c = (a + b) / 2
    if ch.startswith("leg") or ch.startswith("rear"): c = Vector((c.x, c.y, ctr.z))
    off = Vector((c.x - ctr.x, c.y - ctr.y, 0))
    yaw = yaw_of(off) if off.length > 0.05 else 30
    dch = b - a
    if not (ch.startswith("leg") or ch.startswith("rear")) and dch.length > 1e-4 and abs(dch.normalized().dot(FRONT)) > 0.7:
        yaw = 70   # a chain along the body (spine, neck, tail) bends in the side view; three quarter from the left
    if (ch.startswith("leg") or ch.startswith("rear")) and posing.leg_axis(CHAINS[ch]) is not None:
        yaw = 90 if off.dot(LEFT) >= 0 else -90   # a leg that swings fore and aft reads from the side
    if ch.startswith("spike") or ch == "body": yaw += 60
    fnp = f"{outd}/{tag}_{ch}_pose.png"; fnr = f"{outd}/{tag}_{ch}_rest.png"
    worst_txt = (f"rigid>4mm: {len(rig)} max {rig[0][0]*1000:.0f}mm" if rig else "rigid ok") + f"  gap {gap*1000:.0f}mm  str {smax:.2f}"
    shot(fnp, ctr, FAR, yaw, 25, res=(720, 600), color="VERTEX", ortho=WIDE)
    reset()
    shot(fnr, ctr, FAR, yaw, 25, res=(720, 600), color="VERTEX", ortho=WIDE)
    labels[fnp] = f"{tag} {ch} POSED  {worst_txt}"; labels[fnr] = f"{tag} {ch} rest"
    tiles += [fnr, fnp]
    print("POSE", ch, "seam_gap_mm=%.1f stretch_p99=%.2f max=%.2f at" % (gap * 1000, s99, smax), sat, "rigid>4mm:", len(rig), [(r["n"], r["err"], r["at"]) for r in report[ch]["rigid"][:4]], "| art:", [(r["n"], r["err"]) for r in report[ch]["articulated"][:3]])
reset()
json.dump(report, open(f"{outd}/{tag}_report.json", "w"), indent=1)
open(f"{outd}/{tag}_tiles.txt", "w").write(" ".join(tiles))
montage([tiles[i:i + 4] for i in range(0, len(tiles), 4)], f"{outd}/{tag}_sheet.png", labels=labels)   # two chains per row: rest | posed | rest | posed
print("SHEET", f"{outd}/{tag}_sheet.png")
