"""verify.py: byte-level and Blender re-import checks of an output GLB against its source.
Usage: Blender -b --factory-startup -P verify.py -- src.glb out.glb [out.json]
Reports which accessors/bytes differ, influences per vertex, weight sums, zero-weight verts,
image bytes/mime, animation names after re-import, object list. With out.json it writes the same as JSON."""
import bpy, sys, os, json
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np, glbio
args = sys.argv[sys.argv.index("--") + 1:]
src, out = args[:2]
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
changed = [att.get(i, i) for i in names]
print("VERIFY changed accessors", changed, "changed bufferViews", bvs)
images_same = True
for img in a.json.get("images", []):
    if "bufferView" in img:
        bv = a.json["bufferViews"][img["bufferView"]]; o = bv.get("byteOffset", 0)
        same = not diff[o:o + bv["byteLength"]].any()
        images_same = images_same and same
        print("VERIFY image", img.get("mimeType"), "image bytes identical", same)
W = glbio.dense(b)
print("VERIFY max influences", (W > 0).sum(1).max(), "sum min/max", W.sum(1).min().round(6), W.sum(1).max().round(6), "zero-weight verts", (W.sum(1) < 1e-6).sum(), "tris", len(b.idx))
def rig(g):
    nodes = g.json["nodes"]
    return {nodes[j].get("name", str(j)): (g.parent[k], [round(x, 6) for x in g.jpos[k]]) for k, j in enumerate(g.joint_nodes)}
ra, rb = rig(a), rig(b)
same_names = set(ra) == set(rb)
same_parents = same_names and all(ra[n][0] == rb[n][0] for n in ra)
moved = [n for n in ra if same_names and ra[n][1] != rb[n][1]]
print("VERIFY bones same names", same_names, "same parents", same_parents, "moved joints", len(moved))
bpy.ops.import_scene.gltf(filepath=out)
actions = [x.name for x in bpy.data.actions]
bone_count = len(next(o for o in bpy.data.objects if o.type == "ARMATURE").data.bones)
print("VERIFY reimport objects", sorted((o.type, len(o.data.vertices) if o.type == "MESH" else 0) for o in bpy.data.objects), "actions", actions, "bones", bone_count)
if len(args) > 2:
    json.dump(dict(changed=[str(c) for c in changed], accessors=len(a.json["accessors"]), images_same=images_same, max_influences=int((W > 0).sum(1).max()),
                   sum_min=float(W.sum(1).min()), sum_max=float(W.sum(1).max()), zero_weight=int((W.sum(1) < 1e-6).sum()), tris=int(len(b.idx)), same_names=same_names,
                   same_parents=same_parents, moved_joints=moved, bones=bone_count, actions=actions), open(args[2], "w"), indent=1)
