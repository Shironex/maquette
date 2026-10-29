"""verify.py: byte-level and Blender re-import checks of an output GLB against its source.
Usage: Blender -b --factory-startup -P verify.py -- src.glb out.glb
Reports which accessors/bytes differ, influences per vertex, weight sums, zero-weight verts,
image bytes/mime, animation names after re-import, object list."""
import bpy, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np, glbio
src, out = sys.argv[sys.argv.index("--") + 1:]
a, b = glbio.load(src), glbio.load(out)
print("VERIFY", os.path.basename(out), "size", len(b.raw), "json same", a.json == b.json, "bin len same", len(a.bin) == len(b.bin))
diff = np.frombuffer(a.bin, np.uint8) != np.frombuffer(b.bin, np.uint8)
names = []
for i, acc in enumerate(a.json["accessors"]):
    off, dt, n, cnt = glbio._acc_view(a, i)
    if diff[off:off + cnt * n * dt.itemsize].any():
        names.append(i)
bvs = []
for i, bv in enumerate(a.json["bufferViews"]):
    o = bv.get("byteOffset", 0)
    if diff[o:o + bv["byteLength"]].any(): bvs.append(i)
att = {v: k for k, v in a.prim["attributes"].items()}; att[a.prim["indices"]] = "indices"
att[a.json["skins"][0]["inverseBindMatrices"]] = "IBM"
print("VERIFY changed accessors", [att.get(i, i) for i in names], "changed bufferViews", bvs)
img = a.json["images"][0]; print("VERIFY image", img["mimeType"], "image bytes identical", not diff[a.json["bufferViews"][img["bufferView"]].get("byteOffset", 0):][:a.json["bufferViews"][img["bufferView"]]["byteLength"]].any())
W = glbio.dense(b)
print("VERIFY max influences", (W > 0).sum(1).max(), "sum min/max", W.sum(1).min().round(6), W.sum(1).max().round(6), "zero-weight verts", (W.sum(1) < 1e-6).sum(), "tris", len(b.idx))
bpy.ops.import_scene.gltf(filepath=out)
print("VERIFY reimport objects", sorted((o.type, len(o.data.vertices) if o.type == "MESH" else 0) for o in bpy.data.objects), "actions", [x.name for x in bpy.data.actions], "bones", len(next(o for o in bpy.data.objects if o.type == "ARMATURE").data.bones))
