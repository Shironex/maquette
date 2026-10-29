# piecemap.py: colour each welded piece (edge islands merged across co-located split verts) with a
# random colour; optional size list highlights only pieces with those vertex counts.
# Usage: -- in.glb outprefix [n1,n2,...]
import bpy, sys, math, random; sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(sys.argv[sys.argv.index("--")-1])))
from shlib import *
a = sys.argv[sys.argv.index("--")+1:]; glb, out = a[:2]; hl = set(int(x) for x in a[2].split(",")) if len(a) > 2 else None
arm, me, stray = load(glb)
for s in stray: s.hide_render = True
m = me.data; N = len(m.vertices); par = list(range(N))
def f(x):
    while par[x] != x: par[x] = par[par[x]]; x = par[x]
    return x
def u(x, y):
    x, y = f(x), f(y)
    if x != y: par[x] = y
for e in m.edges: u(*e.vertices)
key = {}
for v in m.vertices:
    k = tuple(round(c, 5) for c in v.co)
    if k in key: u(v.index, key[k])
    else: key[k] = v.index
root = [f(i) for i in range(N)]
size = {}
for r in root: size[r] = size.get(r, 0) + 1
random.seed(3); col = {}
attr = m.color_attributes.new("WCOL", "FLOAT_COLOR", "POINT"); m.color_attributes.active_color = attr
for i, r in enumerate(root):
    if r not in col:
        if hl is None or size[r] in hl:
            import colorsys; col[r] = (*colorsys.hsv_to_rgb(random.random(), 0.8, 0.95), 1)
        else: col[r] = (0.8, 0.8, 0.8, 1)
    attr.data[i].color = col[r]
lo, hi = bounds(me); ctr = (lo + hi) / 2
for name, (yw, pt) in VIEWS.items():
    shot(f"{out}_{name}.png", ctr, 2.0, yw, pt, res=(700, 600), color="VERTEX", ortho=1.15, text=f"pieces {name}" + (f" highlight {a[2]}" if hl else ""))
