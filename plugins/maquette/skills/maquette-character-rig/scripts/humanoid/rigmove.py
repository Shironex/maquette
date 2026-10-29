"""rigmove.py: OPTIONAL rig fix for a limb whose joints Tripo placed off its mesh.

Moves the joints listed under "registered_joints" in a fixweights report (only limbs marked
misplaced) to where the mesh's own elbow/wrist/fingers are, keeping every node's rotation and
scale, then rewrites those joints' inverseBindMatrices. Weights are not touched (run it on the
fixweights output). This CHANGES THE REST POSE of that limb, so clips with per-bone translation
tracks for those bones would pull the joints back; use only with the owner's approval.

Usage: blender -b --factory-startup -P rigmove.py -- in.glb report.json out.glb
"""
import sys, os, json
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np, glbio


def q2m(q):
    x, y, z, w = q
    return np.array([[1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
                     [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
                     [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)]])


def local(nd):
    if "matrix" in nd:
        return np.array(nd["matrix"]).reshape(4, 4).T
    m = np.eye(4)
    m[:3, :3] = q2m(nd.get("rotation", [0, 0, 0, 1])) * np.array(nd.get("scale", [1, 1, 1]))
    m[:3, 3] = nd.get("translation", [0, 0, 0])
    return m


src, rep, dst = (sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:])[:3]
g = glbio.load(src)
j = g.json
rep = json.load(open(rep))
moves = {}
for L in rep["limbs"].values():
    if L.get("misplaced"):
        moves.update(L["registered_joints"])
parent = {}
for i, nd in enumerate(j["nodes"]):
    for c in nd.get("children", []):
        parent[c] = i
def world(i):
    m = local(j["nodes"][i])
    while i in parent:
        i = parent[i]; m = local(j["nodes"][i]) @ m
    return m
name2node = {nd.get("name"): i for i, nd in enumerate(j["nodes"])}
old_world = {n: world(name2node[n]) for n in g.names}
# move parents before children
def depth(i):
    d = 0
    while i in parent:
        i = parent[i]; d += 1
    return d
for n in sorted(moves, key=lambda n: depth(name2node[n])):
    i = name2node[n]
    pw = world(parent[i])
    lt = np.linalg.inv(pw) @ np.append(np.array(moves[n]), 1.0)
    nd = j["nodes"][i]
    assert "matrix" not in nd
    print(n, "moved", round(float(np.linalg.norm(world(i)[:3, 3] - np.array(moves[n]))) * 100, 2), "cm")
    nd["translation"] = [float(x) for x in lt[:3]]
# rewrite IBMs of every joint whose world matrix changed
skin = j["skins"][0]
off, dt, n_, cnt = glbio._acc_view(g, skin["inverseBindMatrices"])
ibm = np.frombuffer(g.bin, dtype=np.float32, count=cnt * 16, offset=off).reshape(cnt, 16).copy()
changed = 0
for k, jn in enumerate(g.joint_nodes):
    w = world(jn)
    if np.abs(w - old_world[g.names[k]]).max() > 1e-7:
        ibm[k] = np.linalg.inv(w).T.ravel().astype(np.float32); changed += 1
b = bytearray(g.bin)
b[off:off + ibm.nbytes] = ibm.tobytes()
glbio.save_glb(g, j, b, dst)
print("joints with new bind matrices:", changed)
