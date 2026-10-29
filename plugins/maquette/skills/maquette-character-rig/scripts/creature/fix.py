# fix.py: per-piece weight repair for faceted auto-rigged creatures.
# Method: (1) weld edge islands across co-located split verts into real pieces; (2) per piece, find the
# owner (torso / a leg / a spike) from summed weights, ignoring JUNK chains; (3) small pieces become rigid:
# 100% the bone with the largest summed allowed weight; (4) big pieces keep only the owner's bones plus
# torso plus any secondary owner with a large share (welded parts), renormalised, then a few Laplacian
# smoothing passes on the verts that changed so there is no hard cut inside one piece; (5) mirror pieces
# (a right leg from its left twin) are rebuilt by nearest-vertex transfer across X; (6) cap 4 influences, normalise; drop
# unskinned helper meshes; export GLB with the same image (JPEG) and rest pose.
# Usage: -- in.glb out.glb [dry]
import bpy, sys, math; sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(sys.argv[sys.argv.index("--")-1])))
from shlib import *
BONE_OWNER = {b: o for o, bs in OWNERS.items() for b in bs}
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
key = {}
for v in m.vertices:
    k = tuple(round(c, 5) for c in v.co)
    if k in key: u(v.index, key[k]); nbr[v.index].add(key[k]); nbr[key[k]].add(v.index)
    else: key[k] = v.index
P = defaultdict(list)
for i in range(N): P[f(i)].append(i)
co = [me.matrix_world @ v.co for v in m.vertices]
def owner_shares(vs):
    s = defaultdict(float)
    for i in vs:
        for b, w in W[i].items():
            if b in BONE_OWNER: s[BONE_OWNER[b]] += w
    t = sum(s.values()) or 1
    return {k: v / t for k, v in sorted(s.items(), key=lambda kv: -kv[1])}
log = []; info = {}
stats = defaultdict(int)
for r, vs in P.items():
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
        cs = set(changed)
        for it in range(6):   # smooth only the changed verts toward their neighbours (no hard cut)
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
# cap 4, normalise, zero check
for i in range(N):
    w = dict(sorted(W[i].items(), key=lambda kv: -kv[1])[:4]); t = sum(w.values())
    W[i] = {b: x / t for b, x in w.items()} if t > 0 else {FALLBACK_BONE: 1.0}
diff = sum(1 for i in range(N) if any(abs(W[i].get(b, 0) - W0[i].get(b, 0)) > 0.02 for b in set(W[i]) | set(W0[i])))
for row in sorted(log, key=lambda r: -r[0]): print("PIECE", *row)
print("STATS", dict(stats), "verts_changed>0.02:", diff, "of", N)
from collections import Counter
bc = Counter(b for w in W for b in w)
print("BONES_NOW_DRIVING", len(bc), "zero:", sorted(set(gidx) - set(bc)))
if dry: raise SystemExit
for g in me.vertex_groups: g.remove(range(N))
for i in range(N):
    for b, w in W[i].items(): me.vertex_groups[b].add([i], w, "REPLACE")
for s in stray: bpy.data.objects.remove(s, do_unlink=True)
bpy.ops.export_scene.gltf(filepath=out, export_format="GLB", export_image_format="AUTO", export_animations=False,
                          export_skins=True, export_def_bones=False, export_yup=True)
print("EXPORTED", out)
