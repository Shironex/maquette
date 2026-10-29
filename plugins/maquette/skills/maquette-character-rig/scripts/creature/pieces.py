# pieces.py: find real rigid pieces (edge-connected islands merged across split vertices that share a
# position), then per piece report size, centroid, dominant bones and how mixed its weights are.
# Usage: -- in.glb out.json
import bpy, sys; sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(sys.argv[sys.argv.index("--")-1])))
from shlib import *
import json
from collections import Counter, defaultdict
glb, outj = sys.argv[sys.argv.index("--")+1:]
arm, me, stray = load(glb)
m = me.data; N = len(m.vertices)
par = list(range(N))
def f(a):
    while par[a] != a: par[a] = par[par[a]]; a = par[a]
    return a
def u(a, b):
    a, b = f(a), f(b)
    if a != b: par[a] = b
for e in m.edges: u(*e.vertices)
edge_isl = len({f(i) for i in range(N)})
key = {}
for v in m.vertices:
    k = tuple(round(c, 5) for c in v.co)
    if k in key: u(v.index, key[k])
    else: key[k] = v.index
P = defaultdict(list)
for i in range(N): P[f(i)].append(i)
gname = {g.index: g.name for g in me.vertex_groups}
pieces = []
for r, vs in P.items():
    acc = Counter()
    for i in vs:
        for g in m.vertices[i].groups: acc[gname[g.group]] += g.weight
    tot = sum(acc.values())
    co = [me.matrix_world @ m.vertices[i].co for i in vs]
    lo = [min(c[k] for c in co) for k in range(3)]; hi = [max(c[k] for c in co) for k in range(3)]
    top = acc.most_common(4)
    pieces.append(dict(root=r, n=len(vs), ctr=[sum(c[k] for c in co)/len(co) for k in range(3)], lo=lo, hi=hi,
                       ext=max(hi[k]-lo[k] for k in range(3)), bones=[(b, round(w/tot, 3)) for b, w in top]))
pieces.sort(key=lambda p: -p["n"])
print("EDGE_ISLANDS", edge_isl, "WELDED_PIECES", len(pieces))
hist = Counter()
for p in pieces: hist["<10" if p["n"]<10 else "<50" if p["n"]<50 else "<200" if p["n"]<200 else "<1000" if p["n"]<1000 else ">=1000"] += 1
print("SIZES", hist)
for p in pieces[:60]:
    print(p["n"], "ext=%.3f" % p["ext"], "ctr=", tuple(round(x, 3) for x in p["ctr"]), p["bones"])
json.dump(pieces, open(outj, "w"))
