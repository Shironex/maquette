# verify.py: compare two GLBs: object list, vert/face counts, UV/material/image format, bone names and
# rest heads/tails (max delta), influence stats, zero-weight verts, mesh positions (max delta at rest).
import bpy, sys; sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(sys.argv[sys.argv.index("--")-1])))
from shlib import *
a, b = sys.argv[sys.argv.index("--")+1:]
# byte level (when both files have one skinned primitive): which accessors changed, every image's bytes
import os, numpy as np
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(sys.argv[sys.argv.index("--")-1])), "..", "humanoid"))
import glbio
try:
    ga, gb = glbio.load(a), glbio.load(b)
    same_len = len(ga.bin) == len(gb.bin)
    print("BYTES json same:", ga.json == gb.json, "bin length same:", same_len)
    if same_len:
        diff = np.frombuffer(ga.bin, np.uint8) != np.frombuffer(gb.bin, np.uint8)
        att = {v: k for k, v in ga.prim["attributes"].items()}; att[ga.prim["indices"]] = "indices"
        att[ga.json["skins"][0]["inverseBindMatrices"]] = "IBM"
        ch = []
        for i in range(len(ga.json["accessors"])):
            off, dt, n, cnt = glbio._acc_view(ga, i)
            if diff[off:off + cnt * n * dt.itemsize].any(): ch.append(att.get(i, i))
        print("BYTES changed accessors:", ch, "(a weights-only fix changes only JOINTS_0 and WEIGHTS_0)")
        for im in ga.json.get("images", []):
            bv = ga.json["bufferViews"][im["bufferView"]]; o = bv.get("byteOffset", 0)
            print("BYTES image", im.get("name"), im.get("mimeType"), "identical:", not diff[o:o + bv["byteLength"]].any())
    else:
        print("BYTES files differ in layout (a Blender export): only the re-import checks below apply")
except Exception as e:
    print("BYTES check skipped:", e)
def snap(p):
    arm, me, stray = load(p); m = me.data
    infl = [len([g for g in v.groups if g.weight > 1e-4]) for v in m.vertices]
    return dict(objs=sorted((o.name, o.type) for o in bpy.data.objects), nv=len(m.vertices), nf=len(m.polygons),
        uv=[u.name for u in m.uv_layers], mats=[x.name for x in m.materials],
        imgs=[(i.name, tuple(i.size), i.file_format) for i in bpy.data.images],
        bones={x.name: (x.head_local.copy(), x.tail_local.copy(), x.parent and x.parent.name) for x in arm.data.bones},
        co=[tuple(me.matrix_world @ v.co) for v in m.vertices], uvs=np.array([tuple(l.uv) for l in m.uv_layers[0].data]) if m.uv_layers else None,
        maxinfl=max(infl), zero=infl.count(0), ngroups=len(me.vertex_groups),
        wsum=max(abs(sum(g.weight for g in v.groups) - 1) for v in m.vertices))
A, Bq = snap(a), snap(b)
for k in ("objs", "nv", "nf", "uv", "mats", "imgs", "maxinfl", "zero", "ngroups", "wsum"): print("CMP", k, A[k], "|", Bq[k])
print("BONES same names:", set(A["bones"]) == set(Bq["bones"]), "same parents:", all(A["bones"][n][2] == Bq["bones"][n][2] for n in A["bones"]))
print("BONE rest max delta:", max(max((A["bones"][n][0] - Bq["bones"][n][0]).length, (A["bones"][n][1] - Bq["bones"][n][1]).length) for n in A["bones"]))
if A["nv"] == Bq["nv"]:   # same vertex order (a byte patch): compare per vertex, no rounding
    print("REST per-vertex max delta:", float(np.abs(np.array(A["co"]) - np.array(Bq["co"])).max()))
    if A["uvs"] is not None and Bq["uvs"] is not None and A["uvs"].shape == Bq["uvs"].shape:
        print("UV max delta:", float(np.abs(A["uvs"] - Bq["uvs"]).max()))
from mathutils import kdtree
kd = kdtree.KDTree(len(A["co"]))
for i, c in enumerate(A["co"]): kd.insert(c, i)
kd.balance()
print("REST max dist v2->v1 nearest:", max(kd.find(c)[2] for c in Bq["co"]))
