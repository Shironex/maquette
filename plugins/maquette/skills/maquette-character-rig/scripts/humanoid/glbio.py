"""glbio.py: tiny GLB reader/patcher (numpy only) shared by the rigfix scripts.

load(path) -> G with G.json, G.bin, positions/normals/uvs/indices/joints/weights arrays,
joint names (skin order), bind-pose joint world matrices (inverse of the inverseBindMatrices).
save_weights(G, joints, weights, out) writes a copy of the GLB where ONLY the bytes of the
JOINTS_0 and WEIGHTS_0 accessors change (same layout and size), so mesh, UVs, material,
the JPEG, armature, bone names, rest pose and animations stay byte-identical.
"""
import json, struct
import numpy as np

CT = {5120: np.int8, 5121: np.uint8, 5122: np.int16, 5123: np.uint16, 5125: np.uint32, 5126: np.float32}
NC = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}


class G:
    pass


def _acc_view(g, ai):
    a = g.json["accessors"][ai]
    bv = g.json["bufferViews"][a["bufferView"]]
    dt = np.dtype(CT[a["componentType"]])
    n = NC[a["type"]]
    off = bv.get("byteOffset", 0) + a.get("byteOffset", 0)
    stride = bv.get("byteStride", 0) or dt.itemsize * n
    assert stride == dt.itemsize * n, "interleaved buffers not supported"
    return off, dt, n, a["count"]


def read_acc(g, ai):
    off, dt, n, cnt = _acc_view(g, ai)
    arr = np.frombuffer(g.bin, dtype=dt, count=cnt * n, offset=off)
    return arr.reshape(cnt, n) if n > 1 else arr.copy()


def load(path):
    g = G()
    d = open(path, "rb").read()
    g.raw = d
    jl = struct.unpack("<I", d[12:16])[0]
    g.json = json.loads(d[20:20 + jl])
    g.bin_off = 20 + jl + 8
    bl = struct.unpack("<I", d[20 + jl:24 + jl])[0]
    g.bin = d[g.bin_off:g.bin_off + bl]
    prims = [p for m in g.json["meshes"] for p in m["primitives"]]
    skinned = [p for p in prims if "JOINTS_0" in p["attributes"]]
    if len(prims) > 1 and len(skinned) == 1: prims = skinned   # unskinned props beside one skinned mesh
    assert len(prims) == 1, "expected one skinned primitive"
    p = prims[0]
    g.prim = p
    at = p["attributes"]
    g.pos = read_acc(g, at["POSITION"]).astype(np.float64)
    g.nrm = read_acc(g, at["NORMAL"]).astype(np.float64)
    g.uv = read_acc(g, at["TEXCOORD_0"]).astype(np.float64)
    g.idx = read_acc(g, p["indices"]).astype(np.int64).reshape(-1, 3)
    g.joints = read_acc(g, at["JOINTS_0"]).astype(np.int64)
    g.weights = read_acc(g, at["WEIGHTS_0"]).astype(np.float64)
    skin = g.json["skins"][0]
    g.joint_nodes = skin["joints"]
    g.names = [g.json["nodes"][j].get("name", str(j)) for j in g.joint_nodes]
    ibm = read_acc(g, skin["inverseBindMatrices"]).astype(np.float64).reshape(-1, 4, 4).transpose(0, 2, 1)
    g.bind = np.linalg.inv(ibm)  # joint world matrix at bind (column-major fixed by transpose)
    g.jpos = g.bind[:, :3, 3]
    # parent of each joint (index in skin order or -1)
    node_parent = {}
    for i, nd in enumerate(g.json["nodes"]):
        for c in nd.get("children", []):
            node_parent[c] = i
    jindex = {n: i for i, n in enumerate(g.joint_nodes)}
    g.parent = [jindex.get(node_parent.get(n, -1), -1) for n in g.joint_nodes]
    # mesh node world transform must be identity for positions to be in joint space
    return g


def dense(g):
    """Dense (V, J) weight matrix."""
    W = np.zeros((len(g.pos), len(g.names)))
    np.add.at(W, (np.repeat(np.arange(len(g.pos)), 4), g.joints.ravel()), g.weights.ravel())
    return W


def to_top4(W):
    """Dense weights -> (joints u8, weights f32) top 4, normalised."""
    order = np.argsort(-W, axis=1)[:, :4]
    w = np.take_along_axis(W, order, axis=1)
    s = w.sum(1, keepdims=True)
    assert (s > 0).all(), "vertex with zero weight"
    w = w / s
    order = np.where(w > 0, order, 0)
    return order.astype(np.uint8), w.astype(np.float32)


def save_weights(g, joints, weights, out):
    d = bytearray(g.raw)
    for key, arr in (("JOINTS_0", joints), ("WEIGHTS_0", weights)):
        off, dt, n, cnt = _acc_view(g, g.prim["attributes"][key])
        b = np.ascontiguousarray(arr.astype(dt)).tobytes()
        assert len(b) == cnt * n * dt.itemsize
        d[g.bin_off + off:g.bin_off + off + len(b)] = b
    # keep accessor min/max consistent if present
    open(out, "wb").write(bytes(d))


def components(g, weld=1e-5):
    """Connected components over triangles, welding vertices at the same position (UV seams)."""
    key = np.round(g.pos / weld).astype(np.int64)
    _, inv = np.unique(key, axis=0, return_inverse=True)
    inv = inv.ravel()
    n = inv.max() + 1
    par = np.arange(n)

    def find(x):
        r = x
        while par[r] != r:
            r = par[r]
        while par[x] != r:
            par[x], x = r, par[x]
        return r
    t = inv[g.idx]
    for a, b in ((0, 1), (1, 2)):
        for u, v in zip(t[:, a], t[:, b]):
            ru, rv = find(u), find(v)
            if ru != rv:
                par[ru] = rv
    roots = np.array([find(i) for i in range(n)])
    _, lab = np.unique(roots, return_inverse=True)
    return lab.ravel()[inv], inv


def save_glb(g, j, binbytes, out):
    """Write a GLB from a (modified) json dict and bin bytes (bin length must be unchanged)."""
    js = json.dumps(j, separators=(",", ":")).encode()
    js += b" " * ((4 - len(js) % 4) % 4)
    bn = bytes(binbytes)
    bn += b"\0" * ((4 - len(bn) % 4) % 4)
    total = 12 + 8 + len(js) + 8 + len(bn)
    with open(out, "wb") as f:
        f.write(struct.pack("<III", 0x46546C67, 2, total))
        f.write(struct.pack("<II", len(js), 0x4E4F534A)); f.write(js)
        f.write(struct.pack("<II", len(bn), 0x004E4942)); f.write(bn)
