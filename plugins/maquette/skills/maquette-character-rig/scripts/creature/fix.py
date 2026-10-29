# fix.py: per-piece weight repair for faceted auto-rigged creatures.
# Method: (1) weld edge islands across co-located split verts into real pieces; (2) per piece, find the
# owner (torso / a leg / a spike) from summed weights, ignoring JUNK chains; (3) small pieces become rigid:
# 100% the bone with the largest summed allowed weight; (4) big pieces keep only the owner's bones plus
# torso plus any secondary owner with a large share (welded parts), renormalised, then a few Laplacian
# smoothing passes on the verts that changed so there is no hard cut inside one piece; (5) mirror pieces
# (a right leg from its left twin) are rebuilt by nearest-vertex transfer across X; (6) cap 4 influences, normalise; drop
# unskinned helper meshes; export GLB with the same image (JPEG) and rest pose.
# Optional steps, each off unless rigconfig.py sets it (run before the piece rules):
#   REMAP = {bone: [bone, ...]}: a stray secondary bone (a tuft or duplicate the animator will never key)
#       hands each vertex's weight to the listed bones, split by inverse distance (nearest gets most),
#       so that skin follows the main chain without a hard cut.
#   STRAY_DIST = {owner: distance}: weight of that owner's bones on a vertex farther than distance from
#       every one of its bone segments and nearer another owner's bones is removed (a tail bone driving a
#       paw), then refilled from neighbours.
#   PIECE_RULES = False skips the per piece owner, rigid and mirror rules (for a model that is one
#       continuous shell, where every edge island is a UV island of the same surface, not a rigid piece).
#   WELD_PIECES = False keeps edge islands as the pieces instead of welding them across shared positions.
# Vertices that share a position always end with the same weights, so UV seams cannot crack open.
# Output: when the Blender vertex order matches the GLB (checked), only the JOINTS_0 and WEIGHTS_0 bytes
# are rewritten (humanoid/glbio.py), so mesh, UVs, images, rest pose and node names stay byte identical;
# otherwise it falls back to a Blender glTF export and says so.
# Usage: -- in.glb out.glb [dry]
import bpy, sys, math; sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(sys.argv[sys.argv.index("--")-1])))
from shlib import *
import rigconfig as _rc
BONE_OWNER = {b: o for o, bs in OWNERS.items() for b in bs}
REMAP = getattr(_rc, "REMAP", {}); STRAY_DIST = getattr(_rc, "STRAY_DIST", {})
PIECE_RULES = getattr(_rc, "PIECE_RULES", True); WELD = getattr(_rc, "WELD_PIECES", True)
from collections import defaultdict
from mathutils import Vector, kdtree
args = sys.argv[sys.argv.index("--")+1:]; glb, out = args[:2]; dry = len(args) > 2
arm, me, stray = load(glb)
m = me.data; N = len(m.vertices)
gname = {g.index: g.name for g in me.vertex_groups}; gidx = {v: k for k, v in gname.items()}
W = [{gname[g.group]: g.weight for g in v.groups if g.weight > 1e-5} for v in m.vertices]
W0 = [dict(w) for w in W]
par = list(range(N))
def f(x):
    while par[x] != x: par[x] = par[par[x]]; x = par[x]
    return x
def u(x, y):
    x, y = f(x), f(y)
    if x != y: par[x] = y
nbr = defaultdict(set)
for e in m.edges:
    a, b = e.vertices; u(a, b); nbr[a].add(b); nbr[b].add(a)
key = {}; coloc = defaultdict(list)
for v in m.vertices:
    k = tuple(round(c, 5) for c in v.co); coloc[k].append(v.index)
    if k in key:
        nbr[v.index].add(key[k]); nbr[key[k]].add(v.index)
        if WELD: u(v.index, key[k])
    else: key[k] = v.index
coloc = [g for g in coloc.values() if len(g) > 1]
P = defaultdict(list)
for i in range(N): P[f(i)].append(i)
co = [me.matrix_world @ v.co for v in m.vertices]
amw = arm.matrix_world
SEG = {b.name: (amw @ b.head_local, amw @ b.tail_local) for b in arm.data.bones}
def segdist(p, b):
    a, c = SEG[b]; d = c - a; L = d.length_squared
    t = 0.0 if L < 1e-12 else max(0.0, min(1.0, (p - a).dot(d) / L))
    return (p - (a + d * t)).length
def smooth(cs, passes=6, fill=True):
    """Laplacian passes over the vertex set cs (neighbours by edge and by shared position); with fill,
    first keep passing until every vertex in cs has weight again (a stripped vertex takes its neighbours')."""
    cs = set(cs)
    for it in range(400 if fill else passes):
        empty = [i for i in cs if sum(W[i].values()) <= 0]
        if fill and it >= passes and not empty: break
        new = {}
        for i in cs:
            acc = defaultdict(float); n = 0
            for j in list(nbr[i]) + [i]:
                t = sum(W[j].values())
                if t <= 0: continue
                for b, w in W[j].items(): acc[b] += w / t
                n += 1
            if n: new[i] = {b: w / n for b, w in acc.items()}
        for i, w in new.items(): W[i] = w
pre_changed = set(); stats = defaultdict(int); stray_near = defaultdict(int); stray_w = defaultdict(float)
# REMAP: secondary bones hand their weight to the listed bones, nearest first
for jb, cands in REMAP.items():
    for i in range(N):
        w = W[i].pop(jb, 0.0)
        if w > 0:   # split by inverse distance (power 4) so the hand-over between two bones is soft
            k = {b: 1.0 / (segdist(co[i], b) + 0.01) ** 4 for b in cands}; t = sum(k.values())
            for b, x in k.items(): W[i][b] = W[i].get(b, 0.0) + w * x / t
            stats["remapped_verts"] += 1
# STRAY_DIST: weight far from every bone of its owner, on a vertex nearer another owner, is removed
# and refilled from neighbours
for own, dist in STRAY_DIST.items():
    bs = [b for b in OWNERS[own] if b in SEG]
    for i in range(N):
        hit = [b for b in W[i] if b in bs]
        if not hit or min(segdist(co[i], b) for b in bs) <= dist: continue
        near = min(OWNERS, key=lambda o: min((segdist(co[i], b) for b in OWNERS[o] if b in SEG), default=9))
        if near != own:   # far from its owner AND nearer another one (a long spike on its own tail stays)
            for b in hit: W[i].pop(b)
            pre_changed.add(i); stats["stray_verts_" + own] += 1
            stray_near[(own, near)] += 1; stray_w[(own, near)] = max(stray_w[(own, near)], sum(W0[i].get(b, 0) for b in hit))
if pre_changed:
    ring = set(pre_changed)
    for _ in range(2): ring |= {j for i in ring for j in nbr[i]}   # blend the refill into a small ring around it
    smooth(ring)
def owner_shares(vs):
    s = defaultdict(float)
    for i in vs:
        for b, w in W[i].items():
            if b in BONE_OWNER: s[BONE_OWNER[b]] += w
    t = sum(s.values()) or 1
    return {k: v / t for k, v in sorted(s.items(), key=lambda kv: -kv[1])}
log = []; info = {}
for r, vs in (P.items() if PIECE_RULES else []):
    ext = max(max(co[i][k] for i in vs) - min(co[i][k] for i in vs) for k in range(3))
    sh = owner_shares(vs); own = next(iter(sh), "torso")
    junk = sum(w for i in vs for b, w in W[i].items() if b in JUNK) / max(1, sum(sum(W[i].values()) for i in vs))
    welded = [o for o, x in sh.items() if o != own and o.startswith("leg") and x >= SECONDARY]
    info[r] = dict(ext=ext, own=own, junk=junk, ctr=sum((co[i] for i in vs), Vector()) / len(vs), n=len(vs))
    if ext < RIGID_EXT or own.startswith("spike") or (own == "torso" and not welded):
        if own.startswith("spike"): bone = OWNERS[own][0]
        else:
            acc = defaultdict(float)
            for i in vs:
                for b, w in W[i].items():
                    if BONE_OWNER.get(b) == own: acc[b] += w
            bone = max(acc, key=acc.get) if acc else OWNERS[own][0]
        for i in vs: W[i] = {bone: 1.0}
        mode = "rigid:" + bone; stats["rigid_pieces"] += 1
    else:
        allowed = set(OWNERS[own]) | set(TORSO) | {b for o in welded for b in OWNERS[o]}
        changed = []
        for i in vs:
            nw = {b: w for b, w in W[i].items() if b in allowed}
            if sum(W[i].values()) - sum(nw.values()) > 0.01: changed.append(i)
            W[i] = nw
        smooth(changed, fill=False)   # smooth only the changed verts toward their neighbours (no hard cut)
        mode = f"smooth({len(changed)} verts lost foreign weight) owners={[own] + welded}"
        stats["smooth_pieces"] += 1; stats["smooth_changed_verts"] += len(changed)
    info[r]["mode"] = mode
    log.append([r, len(vs), round(ext, 3), own, {k: round(v, 2) for k, v in list(sh.items())[:3]}, round(junk, 2), mode])
# mirror repair: junk-heavy pieces copy the final weights of their cleaner twin across X
M = dict(MIRROR_PAIRS); M.update({v: k for k, v in MIRROR_PAIRS})
for r, d in info.items():
    if d["junk"] < MIRROR_JUNK: continue
    tw = None
    for r2, d2 in info.items():
        mc = d2["ctr"].copy(); mc.x = -mc.x
        if r2 != r and (mc - d["ctr"]).length < 0.012 and abs(d2["n"] - d["n"]) <= 0.15 * d["n"] and d2["junk"] < d["junk"] - 0.1:
            tw = r2; break
    if tw is None: print("MIRROR none for", d["n"], d["mode"]); continue
    src = P[tw]; kd = kdtree.KDTree(len(src))
    for k, i in enumerate(src): kd.insert(co[i], k)
    kd.balance()
    for i in P[r]:
        p = co[i].copy(); p.x = -p.x; _, k, _ = kd.find(p)
        W[i] = {M.get(b, b): w for b, w in W[src[k]].items()}
    stats["mirrored_pieces"] += 1; stats["mirrored_verts"] += len(P[r])
    for row in log:
        if row[0] == r: row[-1] += f" -> MIRRORED from {info[tw]['n']}-vert twin ({info[tw]['mode']})"
log = [row[1:] for row in log]
# shared positions get one weight set (no seam crack), then cap 4, normalise, zero check
for g in coloc:
    acc = defaultdict(float)
    for i in g:
        t = sum(W[i].values()) or 1
        for b, w in W[i].items(): acc[b] += w / t / len(g)
    for i in g: W[i] = dict(acc)
for i in range(N):
    w = dict(sorted(W[i].items(), key=lambda kv: -kv[1])[:4]); t = sum(w.values())
    W[i] = {b: x / t for b, x in w.items()} if t > 0 else {FALLBACK_BONE: 1.0}
diff = sum(1 for i in range(N) if any(abs(W[i].get(b, 0) - W0[i].get(b, 0)) > 0.02 for b in set(W[i]) | set(W0[i])))
for row in sorted(log, key=lambda r: -r[0]): print("PIECE", *row)
print("STATS", dict(stats), "verts_changed>0.02:", diff, "of", N)
from collections import Counter
bc = Counter(b for w in W for b in w)
for (own, near), n in sorted(stray_near.items(), key=lambda kv: -kv[1]):
    print("STRAY", own, "weight on", n, "verts nearest to", near, "max weight there %.2f" % stray_w[(own, near)])
print("BONES_NOW_DRIVING", len(bc), "zero:", sorted(set(gidx) - set(bc)))
if dry: raise SystemExit
import os, numpy as np
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(sys.argv[sys.argv.index("--")-1])), "..", "humanoid"))
import glbio
try:
    g = glbio.load(glb)
    bl = np.array([tuple(v.co) for v in m.vertices]); gl = g.pos[:, [0, 2, 1]] * np.array([1, -1, 1])   # glTF Y-up -> Blender Z-up
    same_order = len(bl) == len(gl) and float(np.abs(bl - gl).max()) < 1e-4
except Exception as e:
    print("BYTE PATCH unavailable:", e); same_order = False
if same_order:
    J = np.zeros((N, 4), dtype=np.int64); Wt = np.zeros((N, 4))
    jix = {n: k for k, n in enumerate(g.names)}
    for i in range(N):
        for k, (b, w) in enumerate(sorted(W[i].items(), key=lambda kv: -kv[1])): J[i, k] = jix[b]; Wt[i, k] = w
    glbio.save_weights(g, J, Wt, out)
    print("EXPORTED (byte patch: only JOINTS_0 and WEIGHTS_0 changed)", out); raise SystemExit
print("WARNING: vertex order differs from the GLB, falling back to a Blender export (images re-encoded, mesh may be re-split)")
for g in me.vertex_groups: g.remove(range(N))
for i in range(N):
    for b, w in W[i].items(): me.vertex_groups[b].add([i], w, "REPLACE")
for s in stray: bpy.data.objects.remove(s, do_unlink=True)
bpy.ops.export_scene.gltf(filepath=out, export_format="GLB", export_image_format="AUTO", export_animations=False,
                          export_skins=True, export_def_bones=False, export_yup=True)
print("EXPORTED", out)
