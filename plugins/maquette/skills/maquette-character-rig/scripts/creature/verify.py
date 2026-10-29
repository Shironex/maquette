# verify.py: compare two GLBs: object list, vert/face counts, UV/material/image format, bone names and
# rest heads/tails (max delta), influence stats, zero-weight verts, mesh positions (max delta at rest).
import bpy, sys; sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(sys.argv[sys.argv.index("--")-1])))
from shlib import *
a, b = sys.argv[sys.argv.index("--")+1:]
def snap(p):
    arm, me, stray = load(p); m = me.data
    infl = [len([g for g in v.groups if g.weight > 1e-4]) for v in m.vertices]
    return dict(objs=sorted((o.name, o.type) for o in bpy.data.objects), nv=len(m.vertices), nf=len(m.polygons),
        uv=[u.name for u in m.uv_layers], mats=[x.name for x in m.materials],
        imgs=[(i.name, tuple(i.size), i.file_format) for i in bpy.data.images],
        bones={x.name: (x.head_local.copy(), x.tail_local.copy(), x.parent and x.parent.name) for x in arm.data.bones},
        co=sorted(tuple(round(c, 4) for c in me.matrix_world @ v.co) for v in m.vertices),
        maxinfl=max(infl), zero=infl.count(0), ngroups=len(me.vertex_groups),
        wsum=max(abs(sum(g.weight for g in v.groups) - 1) for v in m.vertices))
A, Bq = snap(a), snap(b)
for k in ("objs", "nv", "nf", "uv", "mats", "imgs", "maxinfl", "zero", "ngroups", "wsum"): print("CMP", k, A[k], "|", Bq[k])
print("BONES same names:", set(A["bones"]) == set(Bq["bones"]), "same parents:", all(A["bones"][n][2] == Bq["bones"][n][2] for n in A["bones"]))
print("BONE rest max delta:", max(max((A["bones"][n][0] - Bq["bones"][n][0]).length, (A["bones"][n][1] - Bq["bones"][n][1]).length) for n in A["bones"]))
import itertools
print("REST unique positions identical (1e-4):", set(A["co"]) == set(Bq["co"]), len(set(A["co"])), len(set(Bq["co"])))
from mathutils import kdtree
kd = kdtree.KDTree(len(A["co"]))
for i, c in enumerate(A["co"]): kd.insert(c, i)
kd.balance()
print("REST max dist v2->v1 nearest:", max(kd.find(c)[2] for c in Bq["co"]))
