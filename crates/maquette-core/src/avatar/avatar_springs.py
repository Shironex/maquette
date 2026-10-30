"""Find the parts of a rigged character that hang off the body, and give the ones a person picks
joint chains to swing on.

AI made characters arrive as one skinned mesh with a humanoid rig only: a ponytail, twin tails,
the tips of a cape or a hanging charm are weighted to the head, the spine or the chest and move
rigidly. VRM secondary motion (VRMC_springBone) needs chains of joints to simulate. This script
finds the candidates and, for those confirmed, adds the chains and weights the parts to them.

Maquette runs this headless, never with a window, on a copy of the model:

    blender -b --factory-startup --python-exit-code 1 --python avatar_springs.py -- \
        --job job.json

job.json names the model (a VRM rest pose: a T-pose facing +Z, hips on the Y axis), the mode,
where the outputs go and the VRM bone roles, each a VRM humanoid bone name to the joint (node)
name it maps to. Relative paths resolve against the job file's folder.

Finding ("mode": "find") writes the candidates and a sheet with each in its own colour, and
changes nothing:

    {"mode": "find", "input": "/abs/normalized.glb", "candidates": "candidates.json",
     "sheets": {"candidates": "candidates.png"}, "bones": {"hips": "mixamorig:Hips", ...}}

Chaining ("mode": "chain") adds the chains of the confirmed springs, each named by the id and
root bone of a candidate, and the joints it gets (by its length when left out):

    {"mode": "chain", "input": "/abs/normalized.glb", "output": "springs.glb",
     "report": "report.json", "sheets": {"before": "sheet_before.png",
     "after": "sheet_after.png"}, "bones": {...},
     "springs": [{"id": "hair-1", "rootBone": "head", "line": [[x, y, z], ...], "joints": 6}]}

How a part is found, in the bind space of the skin (glTF metres, measured against the height):

1. The body is a capsule along each humanoid bone (hips to head, the legs, the arms, and the head
   up to its top) with a radius per direction around it: a low percentile of the distances of
   the vertices nearest it, and the smaller of a side and its mirror, so a ponytail on one side
   does not widen the body there. Vertices well past that radius hang off the body; those near
   the head's axis stay with the head (the scalp, a hood).
2. Off-body vertices in touching voxels are one part. A part is where it meets the body at its
   top: the bone most of those neighbours are weighted to is its root (head or neck makes it
   hair; the chest, spine or hips cloth, or an accessory when short). Parts on a limb, too
   small, too short or not hanging down from where they meet the body are left out.
3. The centre line walks the part's voxels from where it hangs and averages the vertices in
   bands of that walk, so it follows a ponytail up over the crown and down the back.
4. How sure the finder is grows with how far the part drops for its length, how much of it
   hangs low, how slender it is and how big.
5. Small pieces that sit close to the body (a charm on a belt, a short chain, an earring) are
   too near it for step 1. They are the vertices standing out of the surface around them
   (their distance from the bone less the median of that stretch and direction of the bone),
   in touching voxels, taller than wide and hanging down from where they meet the rest, left
   out on the forearms, hands, shins and feet. Each is a `charm-N` accessory flagged `small`,
   after the other parts, with a chain of 2 to 4 joints, and the sheet adds a close look.

Nothing is applied while finding. A confirmed spring is matched again to the part with the same
root whose centre line its line lies on, then:

- a chain of joints goes along the centre line, evenly spaced, parented to the root bone, plus
  a tip joint at the end (VRM's last joint only gives the chain its length);
- each vertex of the part takes the chain's weight by where it lies along the line: whole at a
  joint's middle, fading to the next between two middles, and fading back into its old weights
  over the first joint and wherever the part touches the rest of the mesh (for a small piece
  over a quarter of its length), so nothing tears;
- at most 4 influences are kept, summing to 1.

Only new nodes, the skin's joints and inverse bind matrices, and JOINTS_0 and WEIGHTS_0 change
(JOINTS_1 and WEIGHTS_1 are cleared for the vertices that changed). The mesh, UVs, textures,
the humanoid bones and the rest pose stay as they were. Sphere and capsule colliders on the
head, neck, chest, hips, upper arms and upper legs are sized from the measured body.

The report holds the chains (their joints, root, length and vertices), the colliders in their
bones' space and the groups each chain touches, and the weight checks. The before sheet shows
the parts and the joints to come; the after sheet the sway test: rest, the chains bent left,
right and back, front, side, back and a close look where the first part hangs from, plus a near
look at the first small piece when one is chained.
"""

import argparse
import heapq
import json
import math
import os
import struct
import sys
import traceback

import numpy as np

TAG = "[avatar]"
MAX_INFLUENCES = 4

# Tunables, in shares of the model's height unless said otherwise.
SECTORS = 8          # directions around each body bone the body radius is read in
BODY_PCT = 15.0      # percentile of distances that is the body surface in a direction
OFF_SCALE = 1.4      # a vertex past this many body radii...
OFF_PAD = 0.015      # ...plus this pad hangs off the body
HEAD_ZONE = 0.13     # off-body vertices this near the head's axis stay with the head
HEAD_TOP = 0.1       # the head bone reaches this far above the head joint
VOXEL = 0.012        # off-body vertices this near each other are one part
MIN_SHARE = 0.003    # a part needs this share of the vertices...
MIN_LENGTH = 0.06    # ...and this length
HANG_MIN = 0.45      # and drop at least this share of its length below where it hangs from
TOP_BAND = 0.2       # the attachment is read in this top share of a part
LINE_POINTS = 16     # points on a part's centre line
ROOTS = ("head", "neck", "upperChest", "chest", "spine", "hips")
LONG_CLOTH = 0.2     # a part from the trunk this long is cloth, shorter an accessory
JOINT_SPACING = 0.075  # a chain gets a joint about this far apart...
JOINTS_MIN, JOINTS_MAX = 2, 8   # ...within these counts
RAMP = 1.0           # joints over which a part fades from its old weights into the chain
BAND = 0.04          # surface distance over which a part touching the rest fades back
MATCH = 0.05         # a confirmed line this near a part's centre line is that part
# Small pieces: charms, chains and earrings that sit close to the body.
SMALL_RISE = 0.006   # a vertex standing this far out of the surface around it...
SMALL_VOXEL = 0.006  # ...and this near the others is one small piece
SMALL_BIN = 0.03     # the surface around a vertex: this long along its bone...
SMALL_ANGLES = 12    # ...in this many directions around it
SMALL_AROUND = 5     # the surface is read where this many of the nine bins around hold vertices
SMALL_MIN = 8        # a small piece has at least this many vertices...
SMALL_MAX = 0.01     # ...and at most this share of them (or 64 on a coarse mesh)
SMALL_TALL = (0.025, 0.15)  # it is this tall (share of the height)...
SMALL_SLENDER = 1.4  # ...and this many times taller than wide
SMALL_TOP = 0.15     # it hangs from this top share of it
SMALL_JOINTS = (2, 4)  # a small piece's chain gets this many joints
SMALL_BAND = 0.25    # share of its length over which it fades back where it touches the rest
SMALL_SKIP = ("LowerArm", "Hand", "LowerLeg", "Foot", "Toes")  # bones whose pieces are left out
SPRING_PREFIX = "spring_"
COLOURS = ["#e8553f", "#3fa7e8", "#f2c230", "#5fd068", "#c86ce0", "#f08a3c", "#35d0c2",
           "#e85fa0"]
CELL = 400
SWAY_DEG = 50.0
NEAR_LOOK = 0.16     # a near look on the sheets frames this share of the height
# The sway test's bends, each toward a direction in glTF axes.
SWAYS = [("left", (1.0, 0.0, 0.0)), ("right", (-1.0, 0.0, 0.0)), ("back", (0.0, 0.0, -1.0))]


def log(text):
    print(f"{TAG} {text}", flush=True)


def fail(text):
    print(f"{TAG} Error: {text}", flush=True)
    sys.exit(1)


class Refused(Exception):
    """A model this script cannot work on safely; the message says why."""


# glTF container: read the accessors, add nodes and write the skin weights.

COMPONENTS = {5120: np.int8, 5121: np.uint8, 5122: np.int16, 5123: np.uint16,
              5125: np.uint32, 5126: np.float32}
WIDTH = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}
NORMALISED_MAX = {5121: 255.0, 5123: 65535.0, 5120: 127.0, 5122: 32767.0}


class Glb:
    def __init__(self, data):
        if len(data) < 20 or data[:4] != b"glTF":
            raise Refused("this is not a binary glTF (.glb) file")
        length = struct.unpack("<I", data[12:16])[0]
        if data[16:20] != b"JSON":
            raise Refused("the first chunk of the file is not JSON")
        self.json = json.loads(data[20:20 + length])
        self.data = bytearray(data)
        self.bin_at = None
        rest = 20 + length
        if rest + 8 <= len(data) and data[rest + 4:rest + 8] == b"BIN\x00":
            self.bin_at = rest + 8
        for b in self.json.get("buffers", [])[:1]:
            if "uri" in b:
                raise Refused("the model keeps its data outside the .glb")
        if len(self.json.get("buffers", [])) > 1:
            raise Refused("the model has more than one buffer")
        if any("meshopt" in e or "draco" in e for e in self.json.get("extensionsUsed", [])):
            raise Refused("the model is compressed; reweight the uncompressed one")

    def view(self, index):
        acc = self.json["accessors"][index]
        if "sparse" in acc:
            raise Refused("sparse accessors are not supported")
        if "bufferView" not in acc:
            raise Refused("an accessor without data")
        bv = self.json["bufferViews"][acc["bufferView"]]
        dtype = np.dtype(COMPONENTS[acc["componentType"]]).newbyteorder("<")
        width = WIDTH[acc["type"]]
        offset = self.bin_at + bv.get("byteOffset", 0) + acc.get("byteOffset", 0)
        stride = bv.get("byteStride") or dtype.itemsize * width
        return acc, dtype, width, offset, stride

    def read(self, index):
        acc, dtype, width, offset, stride = self.view(index)
        count = acc["count"]
        if count == 0:
            return np.zeros((0, width))
        out = np.ndarray((count, width), dtype, self.data, offset, (stride, dtype.itemsize))
        out = np.array(out, np.float64)
        if acc.get("normalized") and acc["componentType"] in NORMALISED_MAX:
            out /= NORMALISED_MAX[acc["componentType"]]
        return out

    def write(self, index, values):
        acc, dtype, width, offset, stride = self.view(index)
        count = acc["count"]
        if values.shape != (count, width):
            raise Refused("an accessor changed size")
        out = np.ndarray((count, width), dtype, self.data, offset, (stride, dtype.itemsize))
        ct = acc["componentType"]
        if dtype.kind == "f":
            out[...] = values.astype(dtype)
        elif acc.get("normalized") and ct in NORMALISED_MAX:
            out[...] = quantise(values, NORMALISED_MAX[ct]).astype(dtype)
        else:
            top = np.iinfo(dtype).max
            if values.size and values.max() > top:
                raise Refused(f"a joint index above {top} does not fit this model's joints")
            out[...] = np.rint(values).astype(dtype)


def quantise(weights, top):
    """Rounds rows of weights to integers that still sum to `top`."""
    scaled = weights * top
    base = np.floor(scaled)
    short = np.rint(top - base.sum(1)).astype(int)
    order = np.argsort(-(scaled - base), axis=1)
    for k in range(weights.shape[1]):
        add = short > k
        rows = np.nonzero(add)[0]
        base[rows, order[rows, k]] += 1
    return base


def node_worlds(doc):
    nodes = doc.get("nodes", [])
    parent = [-1] * len(nodes)
    for i, n in enumerate(nodes):
        for c in n.get("children", []):
            parent[c] = i
    local = []
    for n in nodes:
        if "matrix" in n:
            local.append(np.array(n["matrix"], np.float64).reshape(4, 4).T)
            continue
        t = np.array(n.get("translation", [0, 0, 0]), np.float64)
        x, y, z, w = n.get("rotation", [0, 0, 0, 1])
        s = np.array(n.get("scale", [1, 1, 1]), np.float64)
        r = np.array([
            [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
            [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
            [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)],
        ])
        m = np.eye(4)
        m[:3, :3] = r * s
        m[:3, 3] = t
        local.append(m)
    world = [None] * len(nodes)

    def get(i):
        if world[i] is None:
            world[i] = local[i] if parent[i] < 0 else get(parent[i]) @ local[i]
        return world[i]

    return [get(i) for i in range(len(nodes))], parent


class Skin:
    """One skin's primitives as one vertex list, with its joints in bind space."""

    def __init__(self, glb, skin_index, prims, worlds, parent):
        doc = glb.json
        skin = doc["skins"][skin_index]
        self.joint_nodes = list(skin["joints"])
        self.names = [doc["nodes"][n].get("name", f"node {n}") for n in self.joint_nodes]
        self.index = skin_index
        if "inverseBindMatrices" in skin:
            ibm = glb.read(skin["inverseBindMatrices"]).reshape(-1, 4, 4).transpose(0, 2, 1)
            bind = np.linalg.inv(ibm)
        else:
            # No inverse bind matrices means identity ones; the joints sit where the nodes do.
            ibm = np.tile(np.eye(4), (len(self.joint_nodes), 1, 1))
            bind = np.array([worlds[n] for n in self.joint_nodes])
        self.ibm = list(ibm)
        self.at = bind[:, :3, 3]
        position = {n: i for i, n in enumerate(self.joint_nodes)}
        # Each joint's nearest ancestor that is a joint of this skin.
        self.parent = []
        for n in self.joint_nodes:
            p = parent[n]
            while p >= 0 and p not in position:
                p = parent[p]
            self.parent.append(position.get(p, -1))
        self.prims = []
        pos, joints, weights, tris = [], [], [], []
        start = 0
        for prim in prims:
            at = prim["attributes"]
            p = glb.read(at["POSITION"])
            sets = [k for k in (0, 1) if f"JOINTS_{k}" in at and f"WEIGHTS_{k}" in at]
            j = np.concatenate([glb.read(at[f"JOINTS_{k}"]) for k in sets], 1).astype(np.int64)
            w = np.concatenate([glb.read(at[f"WEIGHTS_{k}"]) for k in sets], 1)
            if "indices" in prim:
                idx = glb.read(prim["indices"]).astype(np.int64).ravel()
            else:
                idx = np.arange(len(p))
            mode = prim.get("mode", 4)
            if mode == 4:
                t = idx[: len(idx) // 3 * 3].reshape(-1, 3)
            elif mode == 5:
                t = np.stack([idx[:-2], idx[1:-1], idx[2:]], 1)
            elif mode == 6:
                t = np.stack([np.full(len(idx) - 2, idx[0]), idx[1:-1], idx[2:]], 1)
            else:
                t = np.zeros((0, 3), np.int64)
            self.prims.append((prim, sets, start, len(p)))
            pos.append(p)
            joints.append(j)
            weights.append(w)
            tris.append(t + start)
            start += len(p)
        k = max(x.shape[1] for x in joints)
        self.pos = np.concatenate(pos)
        self.joints = np.concatenate([np.pad(x, ((0, 0), (0, k - x.shape[1]))) for x in joints])
        self.weights = np.concatenate([np.pad(x, ((0, 0), (0, k - x.shape[1]))) for x in weights])
        self.tris = np.concatenate(tris) if tris else np.zeros((0, 3), np.int64)
        self.weights[self.joints >= len(self.joint_nodes)] = 0.0
        self.joints[self.joints >= len(self.joint_nodes)] = 0


def skins_of(glb):
    """Each skin's primitives with skin weights, each accessor set read once."""
    doc = glb.json
    worlds, parent = node_worlds(doc)
    by_skin = {}
    seen = set()
    for node in doc.get("nodes", []):
        if "mesh" not in node or "skin" not in node:
            continue
        for prim in doc["meshes"][node["mesh"]]["primitives"]:
            at = prim.get("attributes", {})
            if not all(k in at for k in ("POSITION", "JOINTS_0", "WEIGHTS_0")):
                continue
            key = (at["POSITION"], at["JOINTS_0"], at["WEIGHTS_0"], prim.get("indices"))
            if key in seen:
                continue
            seen.add(key)
            by_skin.setdefault(node["skin"], []).append(prim)
    return [Skin(glb, s, prims, worlds, parent) for s, prims in sorted(by_skin.items())]


# Geometry helpers.


# Geometry.


def smoothstep(x):
    x = np.clip(x, 0.0, 1.0)
    return x * x * (3.0 - 2.0 * x)


def unit(v):
    n = np.linalg.norm(v)
    return v / n if n > 1e-12 else v


def segment_distance(points, a, b):
    ab = b - a
    t = np.clip(((points - a) @ ab) / max(float(ab @ ab), 1e-12), 0.0, 1.0)
    return np.linalg.norm(points - (a + t[:, None] * ab), axis=1)


class Welded:
    """The mesh with vertices at the same place (split for UV seams) as one."""

    def __init__(self, pos, tris, eps):
        key = np.round(pos / eps).astype(np.int64)
        _, inv = np.unique(key, axis=0, return_inverse=True)
        self.of = inv.ravel()
        self.n = int(self.of.max()) + 1 if len(self.of) else 0
        self.pos = np.zeros((self.n, 3))
        self.pos[self.of] = pos
        t = self.of[tris]
        t = t[(t[:, 0] != t[:, 1]) & (t[:, 1] != t[:, 2]) & (t[:, 2] != t[:, 0])]
        self.tris = t
        pairs = np.sort(np.stack([t[:, [0, 1]], t[:, [1, 2]], t[:, [2, 0]]], 1), 2)
        flat = pairs.reshape(-1, 2)
        self.edges, which = np.unique(flat, axis=0, return_inverse=True)
        self.tri_edges = which.ravel().reshape(-1, 3)
        self.length = np.linalg.norm(self.pos[self.edges[:, 0]] - self.pos[self.edges[:, 1]],
                                     axis=1)


def components(n, edges):
    parent = np.arange(n)

    def find(x):
        root = x
        while parent[root] != root:
            root = parent[root]
        while parent[x] != root:
            parent[x], x = root, parent[x]
        return root

    for a, b in edges.tolist():
        ra, rb = find(a), find(b)
        if ra != rb:
            parent[ra] = rb
    roots = np.array([find(i) for i in range(n)], np.int64)
    _, label = np.unique(roots, return_inverse=True)
    return label.ravel()


def surface_distance(n, edges, length, sources, limit):
    """Shortest distance over the welded mesh from any source, up to `limit` (inf beyond)."""
    adj = [[] for _ in range(n)]
    for (a, b), l in zip(edges.tolist(), length.tolist()):
        adj[a].append((b, l))
        adj[b].append((a, l))
    dist = np.full(n, np.inf)
    heap = []
    for s in np.nonzero(sources)[0].tolist():
        dist[s] = 0.0
        heap.append((0.0, s))
    heapq.heapify(heap)
    while heap:
        d, u = heapq.heappop(heap)
        if d > dist[u]:
            continue
        for v, l in adj[u]:
            nd = d + l
            if nd < dist[v] and nd < limit:
                dist[v] = nd
                heapq.heappush(heap, (nd, v))
    return dist


def top_four(joints, weights):
    """Merges repeated joints, keeps the 4 largest and normalises."""
    j = joints.copy()
    w = weights.copy()
    k = j.shape[1]
    for a in range(k):
        for b in range(a + 1, k):
            same = (j[:, a] == j[:, b]) & (w[:, b] > 0)
            w[same, a] += w[same, b]
            w[same, b] = 0.0
    order = np.argsort(-w, axis=1, kind="stable")[:, :MAX_INFLUENCES]
    j = np.take_along_axis(j, order, 1)
    w = np.take_along_axis(w, order, 1)
    if w.shape[1] < MAX_INFLUENCES:
        pad = MAX_INFLUENCES - w.shape[1]
        j = np.pad(j, ((0, 0), (0, pad)))
        w = np.pad(w, ((0, 0), (0, pad)))
    total = w.sum(1, keepdims=True)
    w = np.where(total > 0, w / np.maximum(total, 1e-30), w)
    j = np.where(w > 0, j, 0)
    return j, w


def weight_checks(joints, weights):
    total = weights.sum(1)
    count = (weights > 0).sum(1)
    return {
        "vertices": int(len(weights)),
        "maxInfluences": int(count.max()) if len(count) else 0,
        "unweighted": int((total <= 0).sum()),
        "offSum": int((np.abs(total - 1.0) > 1e-3).sum()),
    }


# Finding the parts.


BODY_SEGMENTS = [
    ("hips", "spine"), ("spine", "chest"), ("chest", "upperChest"), ("upperChest", "neck"),
    ("chest", "neck"), ("neck", "head"),
    ("hips", "leftUpperLeg"), ("hips", "rightUpperLeg"),
    ("leftUpperLeg", "leftLowerLeg"), ("leftLowerLeg", "leftFoot"), ("leftFoot", "leftToes"),
    ("rightUpperLeg", "rightLowerLeg"), ("rightLowerLeg", "rightFoot"),
    ("rightFoot", "rightToes"),
    ("upperChest", "leftShoulder"), ("chest", "leftShoulder"),
    ("leftShoulder", "leftUpperArm"), ("leftUpperArm", "leftLowerArm"),
    ("leftLowerArm", "leftHand"),
    ("upperChest", "rightShoulder"), ("chest", "rightShoulder"),
    ("rightShoulder", "rightUpperArm"), ("rightUpperArm", "rightLowerArm"),
    ("rightLowerArm", "rightHand"),
]


def radius_cap(name):
    """The widest a body bone may be read, as a share of the height."""
    a, b = name.split(">")
    if b == "top":
        return 0.06
    if b == "head":
        return 0.05
    if any(k in b for k in ("Foot", "Toes")):
        return 0.04
    if any(k in b for k in ("Leg",)):
        return 0.045
    if any(k in b for k in ("Shoulder", "Arm", "Hand")):
        return 0.05
    return 0.08


def mirror_role(role):
    if role.startswith("left"):
        return "right" + role[4:]
    if role.startswith("right"):
        return "left" + role[5:]
    return role


class BodyModel:
    """The body as capsules along the humanoid bones, each with a radius per direction read
    from the mesh: a low percentile of the distances, and the smaller of a side and its mirror,
    so hair or cloth hanging on one side does not widen the body."""

    def __init__(self, skin, bones):
        by_name = {}
        for i, name in enumerate(skin.names):
            by_name.setdefault(name, i)
        self.skin = skin
        self.joint = {r: by_name[n] for r, n in bones.items() if n in by_name}
        for role in ("hips", "spine", "head", "leftUpperLeg", "rightUpperLeg",
                     "leftUpperArm", "rightUpperArm"):
            if role not in self.joint:
                raise Refused(f"the bone map has no {role} in this skin")
        pos = skin.pos
        self.height = float(pos[:, 1].max() - pos[:, 1].min()) if len(pos) else 0.0
        if self.height <= 1e-9:
            raise Refused("the skinned mesh has no height")
        h = self.height
        segs = []
        seen = set()
        for a, b in BODY_SEGMENTS:
            if a in self.joint and b in self.joint and b not in seen:
                # The first segment into a bone wins (chest to neck only without an upper chest).
                segs.append((a, b))
                seen.add(b)
        self.names = [f"{a}>{b}" for a, b in segs]
        self.roles = [a for a, _ in segs]
        starts = [self.at(a) for a, _ in segs]
        ends = [self.at(b) for _, b in segs]
        head = self.at("head")
        self.names.append("head>top")
        self.roles.append("head")
        starts.append(head)
        ends.append(head + np.array([0.0, HEAD_TOP * h, 0.0]))
        self.a = np.array(starts)
        self.d = np.array(ends) - self.a
        self.len2 = np.maximum((self.d * self.d).sum(1), 1e-12)
        self.frames = []
        for d in self.d:
            ax = unit(d)
            ref = np.array([0.0, 0.0, 1.0]) if abs(ax[2]) < 0.9 else np.array([1.0, 0.0, 0.0])
            u = unit(np.cross(ax, ref))
            self.frames.append((u, np.cross(ax, u)))
        self.head_mid = head + np.array([0.0, 0.5 * HEAD_TOP * h, 0.0])
        seg, dist, sector = self.place(pos)
        own = self.radii(seg, dist, sector)
        mirrored = self.radii(*self.place(pos * np.array([-1.0, 1.0, 1.0])))
        radius = np.fmin(own, mirrored)
        for k in range(len(radius)):
            row = radius[k]
            if np.all(np.isnan(row)):
                radius[k] = 0.04 * h
            else:
                radius[k] = np.where(np.isnan(row), np.nanmedian(row), row)
        caps = np.array([radius_cap(name) * h for name in self.names])
        radius = np.minimum(radius, caps[:, None])
        radius = np.minimum(radius, 2.0 * np.median(radius, axis=1, keepdims=True))
        self.radius = radius
        self.seg, self.dist, self.sector = seg, dist, sector

    def at(self, role):
        return self.skin.at[self.joint[role]]

    def head_distance(self, pts):
        """Distance from the head's axis, neck to the top of the head."""
        top = self.at("head") + np.array([0.0, HEAD_TOP * self.height, 0.0])
        low = self.at("neck") if "neck" in self.joint else self.at("head")
        return segment_distance(pts, low, top)

    def place(self, pts):
        """Each point's nearest body bone, its distance from it and its direction sector."""
        n = len(pts)
        best = np.full(n, np.inf)
        seg = np.zeros(n, np.int64)
        near = np.zeros_like(pts)
        for k in range(len(self.a)):
            t = np.clip(((pts - self.a[k]) @ self.d[k]) / self.len2[k], 0.0, 1.0)
            q = self.a[k] + t[:, None] * self.d[k]
            dd = np.linalg.norm(pts - q, axis=1)
            take = dd < best
            best[take] = dd[take]
            seg[take] = k
            near[take] = q[take]
        sector = np.zeros(n, np.int64)
        rel = pts - near
        for k, (u, v) in enumerate(self.frames):
            m = seg == k
            ang = np.arctan2(rel[m] @ v, rel[m] @ u)
            sector[m] = ((ang + np.pi) / (2 * np.pi) * SECTORS).astype(np.int64) % SECTORS
        return seg, best, sector

    def radii(self, seg, dist, sector):
        out = np.full((len(self.a), SECTORS), np.nan)
        for k in range(len(self.a)):
            for q in range(SECTORS):
                dd = dist[(seg == k) & (sector == q)]
                if len(dd) > 10:
                    out[k, q] = np.percentile(dd, BODY_PCT)
        return out

    def off(self):
        """Vertices hanging off the body."""
        limit = self.radius[self.seg, self.sector] * OFF_SCALE + OFF_PAD * self.height
        return self.dist > limit

    def mean_radius(self, role_a, role_b):
        name = f"{role_a}>{role_b}"
        if name not in self.names:
            return None
        return float(self.radius[self.names.index(name)].mean())


def voxel_components(pts, size):
    """Points in touching voxels as one part: each point's part, and the voxel graph."""
    key = np.floor(pts / size).astype(np.int64)
    cells, of = np.unique(key, axis=0, return_inverse=True)
    of = of.ravel()
    lookup = {tuple(c): i for i, c in enumerate(cells.tolist())}
    edges = []
    for i, (x, y, z) in enumerate(cells.tolist()):
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                for dz in (-1, 0, 1):
                    j = lookup.get((x + dx, y + dy, z + dz))
                    if j is not None and j > i:
                        edges.append((i, j))
    edges = np.array(edges, np.int64).reshape(-1, 2)
    part = components(len(cells), edges)
    return part[of], of, cells, edges


def graph_distance(n, edges, length, sources):
    return surface_distance(n, edges, length, sources, np.inf)


def humanoid_of(skin, joint_role):
    """Each joint's nearest humanoid bone, itself or an ancestor."""
    out = []
    for j in range(len(skin.names)):
        k = j
        while k >= 0 and k not in joint_role:
            k = skin.parent[k]
        out.append(joint_role.get(k))
    return out


def centre_line(pts, sources, size):
    """A part's centre line from where it hangs: the voxels of its points are walked from the
    sources outward, and the points are averaged in bands of that walking distance. Returns the
    line and each point's walking distance as a share of the whole walk."""
    key = np.floor(pts / size).astype(np.int64)
    cells, of = np.unique(key, axis=0, return_inverse=True)
    of = of.ravel()
    lookup = {tuple(c): i for i, c in enumerate(cells.tolist())}
    edges, length = [], []
    for i, (x, y, z) in enumerate(cells.tolist()):
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                for dz in (-1, 0, 1):
                    j = lookup.get((x + dx, y + dy, z + dz))
                    if j is not None and j > i:
                        edges.append((i, j))
                        length.append(math.sqrt(dx * dx + dy * dy + dz * dz))
    edges = np.array(edges, np.int64).reshape(-1, 2)
    src = np.zeros(len(cells), bool)
    src[of[sources]] = True
    walk = surface_distance(len(cells), edges, np.array(length), src, np.inf)
    reach = np.isfinite(walk)
    if not reach.any():
        walk = np.zeros(len(cells))
        reach[:] = True
    walk[~reach] = walk[reach].max()
    far = max(float(np.percentile(walk[of], 98)), 1e-9)
    t = np.clip(walk[of] / far, 0.0, 1.0)
    bands = np.minimum((t * LINE_POINTS).astype(np.int64), LINE_POINTS - 1)
    line = [pts[sources].mean(0)]
    line_t = [0.0]
    for b in range(LINE_POINTS):
        m = bands == b
        if m.any():
            line.append(pts[m].mean(0))
            line_t.append((b + 0.5) / LINE_POINTS)
    line = np.array(line)
    if len(line) > 3:
        smooth = line.copy()
        smooth[1:-1] = (line[:-2] + 2 * line[1:-1] + line[2:]) / 4
        line = smooth
    return line, np.array(line_t), t


def arc(line):
    seg = np.linalg.norm(np.diff(line, axis=0), axis=1)
    return np.concatenate([[0.0], np.cumsum(seg)])


def find_parts(skin, bones):
    """The parts hanging off the body, each with its vertices, root bone, kind, centre line
    and how sure the finder is, most sure first."""
    body = BodyModel(skin, bones)
    h = body.height
    pos = skin.pos
    n = len(pos)
    off = body.off()
    off &= body.head_distance(pos) > HEAD_ZONE * h
    mesh = Welded(pos, skin.tris, 1e-5 * h)
    joint_role = {j: r for r, j in body.joint.items()}
    role_of = humanoid_of(skin, joint_role)
    dominant = skin.joints[np.arange(n), np.argmax(skin.weights, 1)]
    first = np.full(mesh.n, -1)
    first[mesh.of[::-1]] = np.arange(n)[::-1]
    idx = np.nonzero(off)[0]
    parts = []
    if len(idx) == 0:
        return body, parts
    label, _, _, _ = voxel_components(pos[idx], VOXEL * h)
    wedges = mesh.edges
    for lab in np.unique(label):
        members = idx[label == lab]
        if len(members) < MIN_SHARE * n:
            continue
        pts = pos[members]
        top, bottom = float(pts[:, 1].max()), float(pts[:, 1].min())
        inside = np.zeros(mesh.n, bool)
        inside[mesh.of[members]] = True
        cross = inside[wedges[:, 0]] != inside[wedges[:, 1]]
        ends = wedges[cross]
        outer = np.where(inside[ends[:, 0]], ends[:, 1], ends[:, 0])
        inner = np.where(inside[ends[:, 0]], ends[:, 0], ends[:, 1])
        high = mesh.pos[inner][:, 1] >= top - TOP_BAND * (top - bottom)
        votes = {}
        for w in outer[high].tolist():
            role = role_of[dominant[first[w]]]
            if role:
                votes[role] = votes.get(role, 0) + 1
        if not votes:
            # A piece of its own (no edge to the body) hangs from the bone its top is weighted to.
            for v in members[pts[:, 1] >= top - TOP_BAND * (top - bottom)].tolist():
                role = role_of[dominant[v]]
                if role:
                    votes[role] = votes.get(role, 0) + 1
        if high.any():
            seed = np.zeros(mesh.n, bool)
            seed[inner[high]] = True
            sources = np.nonzero(seed[mesh.of[members]])[0]
        else:
            sources = np.nonzero(pts[:, 1] >= top - 0.05 * (top - bottom))[0]
        attach = pts[sources].mean(0)
        if votes:
            root = max(votes, key=votes.get)
        else:
            seg, _, _ = body.place(attach[None])
            root = body.roles[int(seg[0])]
        root = {"leftShoulder": "upperChest", "rightShoulder": "upperChest"}.get(root, root)
        if root not in ROOTS or root not in body.joint:
            continue
        line, line_t, t = centre_line(pts, sources, VOXEL * h)
        length = float(arc(line)[-1])
        drop = float(attach[1] - bottom)
        if length < MIN_LENGTH * h or drop < HANG_MIN * length:
            continue
        across = float(np.linalg.norm(np.ptp(pts[:, [0, 2]], axis=0)))
        below = float(np.median(attach[1] - pts[:, 1])) / max(drop, 1e-9)
        hang = min(drop / max(length, 1e-9), 1.0)
        slender = min(drop / max(across, 1e-9) / 2.0, 1.0)
        size = min(len(members) / (0.02 * n), 1.0)
        confidence = round(0.35 * hang + 0.25 * min(below * 2.0, 1.0) + 0.25 * slender
                           + 0.15 * size, 3)
        if root in ("head", "neck"):
            kind = "hair"
        elif length >= LONG_CLOTH * h:
            kind = "cloth"
        else:
            kind = "accessory"
        parts.append({
            "members": members, "t": t, "line_t": line_t, "root": root, "kind": kind, "line": line,
            "length": length, "drop": drop, "confidence": confidence,
            "measures": {"hang": hang, "below": below, "slender": slender, "size": size},
        })
    parts.sort(key=lambda p: (-p["confidence"], -len(p["members"])))
    return body, parts + find_small(skin, body, parts, role_of, dominant)


def local_rise(body, pos):
    """How far each vertex stands out of the surface around it: its distance from its body
    bone less the median distance of the vertices along that bone and in that direction, read
    in bins with their neighbours so a small piece does not lift its own surface."""
    h = body.height
    n = len(pos)
    along = np.zeros(n)
    angle = np.zeros(n)
    for k in range(len(body.a)):
        m = body.seg == k
        t = ((pos[m] - body.a[k]) @ body.d[k]) / body.len2[k]
        along[m] = t * math.sqrt(body.len2[k])
        rel = pos[m] - (body.a[k] + np.clip(t, 0.0, 1.0)[:, None] * body.d[k])
        u, v = body.frames[k]
        angle[m] = np.arctan2(rel @ v, rel @ u)
    row = np.floor(along / (SMALL_BIN * h)).astype(np.int64) + 1000
    col = (np.floor((angle + np.pi) / (2 * np.pi) * SMALL_ANGLES).astype(np.int64)
           % SMALL_ANGLES)
    key = (body.seg * 4000 + row) * SMALL_ANGLES + col
    cells, of = np.unique(key, return_inverse=True)
    of = of.ravel()
    order = np.argsort(of, kind="stable")
    splits = np.split(body.dist[order], np.cumsum(np.bincount(of))[:-1])
    median = {int(c): float(np.median(d)) for c, d in zip(cells.tolist(), splits)}
    surface = np.full(len(cells), np.nan)
    for i, c in enumerate(cells.tolist()):
        line, a = divmod(c, SMALL_ANGLES)
        near = [median.get((line + dl) * SMALL_ANGLES + (a + da) % SMALL_ANGLES)
                for dl in (-1, 0, 1) for da in (-1, 0, 1)]
        near = [x for x in near if x is not None]
        if len(near) >= SMALL_AROUND:
            surface[i] = np.median(near)
    # Where too little of the mesh lies around (a coarse mesh), the body's radius is the surface.
    surface = surface[of]
    bare = np.isnan(surface)
    surface[bare] = body.radius[body.seg[bare], body.sector[bare]]
    return body.dist - surface


def find_small(skin, body, parts, role_of, dominant):
    """Small pieces hanging close to the body (a charm on a belt, a short chain, an earring):
    vertices standing out of the surface around them, in touching voxels, taller than wide and
    hanging down from where they meet the rest. Each is an accessory with a short chain."""
    h = body.height
    pos = skin.pos
    n = len(pos)
    taken = np.zeros(n, bool)
    for part in parts:
        taken[part["members"]] = True
    skip = [k for k, name in enumerate(body.names)
            if any(word in name.split(">")[0] for word in SMALL_SKIP)]
    rise = local_rise(body, pos)
    idx = np.nonzero((rise > SMALL_RISE * h) & ~taken & ~np.isin(body.seg, skip))[0]
    found = []
    if len(idx) == 0:
        return found
    label, _, _, _ = voxel_components(pos[idx], SMALL_VOXEL * h)
    for lab in np.unique(label):
        members = idx[label == lab]
        if not SMALL_MIN <= len(members) <= max(SMALL_MAX * n, 64):
            continue
        pts = pos[members]
        extent = np.ptp(pts, axis=0)
        wide = float(max(extent[0], extent[2]))
        tall = float(extent[1])
        if not SMALL_TALL[0] * h <= tall <= SMALL_TALL[1] * h or tall < SMALL_SLENDER * wide:
            continue
        top, bottom = float(pts[:, 1].max()), float(pts[:, 1].min())
        sources = np.nonzero(pts[:, 1] >= top - SMALL_TOP * (top - bottom))[0]
        line, line_t, t = centre_line(pts, sources, SMALL_VOXEL * h)
        length = float(arc(line)[-1])
        drop = float(pts[sources].mean(0)[1] - bottom)
        if length < SMALL_TALL[0] * h or drop < HANG_MIN * length:
            continue
        votes = {}
        for v in members[sources].tolist():
            role = role_of[dominant[v]]
            if role:
                votes[role] = votes.get(role, 0) + 1
        root = max(votes, key=votes.get) if votes else None
        if root not in body.joint:
            continue
        hang = min(drop / max(length, 1e-9), 1.0)
        slender = min(tall / max(wide, 1e-9) / 3.0, 1.0)
        stands = min(float(np.median(rise[members])) / (2.5 * SMALL_RISE * h), 1.0)
        confidence = round(0.4 * hang + 0.3 * slender + 0.3 * stands, 3)
        found.append({
            "members": members, "t": t, "line_t": line_t, "root": root, "kind": "accessory",
            "line": line, "length": length, "drop": drop, "confidence": confidence,
            "small": True,
            "measures": {"hang": hang, "slender": slender, "stands": stands},
        })
    found.sort(key=lambda p: (-p["confidence"], -len(p["members"])))
    return found


# Writing the chains into the GLB.


class Grown:
    """A GLB that can take new nodes, accessors and bytes: the JSON is edited, new data goes
    at the end of the BIN chunk, and everything already there keeps its bytes and offsets."""

    def __init__(self, glb):
        self.glb = glb
        self.json = glb.json
        start = glb.bin_at
        length = self.json["buffers"][0]["byteLength"] if self.json.get("buffers") else 0
        self.bin = bytearray(glb.data[start:start + length]) if start is not None else bytearray()

    def view(self, data):
        while len(self.bin) % 4:
            self.bin.append(0)
        offset = len(self.bin)
        self.bin.extend(data)
        if not self.json.get("buffers"):
            self.json["buffers"] = [{}]
        self.json["buffers"][0]["byteLength"] = len(self.bin)
        views = self.json.setdefault("bufferViews", [])
        views.append({"buffer": 0, "byteOffset": offset, "byteLength": len(data)})
        return len(views) - 1

    def accessor(self, array, component, kind):
        dtype = np.dtype(COMPONENTS[component]).newbyteorder("<")
        data = np.ascontiguousarray(array, dtype=dtype).tobytes()
        accessors = self.json.setdefault("accessors", [])
        accessors.append({"bufferView": self.view(data), "componentType": component,
                          "count": int(len(array)), "type": kind})
        return len(accessors) - 1

    def node(self, name, translation, parent):
        nodes = self.json["nodes"]
        nodes.append({"name": name, "translation": [float(x) for x in translation]})
        index = len(nodes) - 1
        nodes[parent].setdefault("children", []).append(index)
        return index

    def bytes(self):
        """The whole file again, its chunks padded to four bytes."""
        self.glb.json = self.json
        text = json.dumps(self.json, separators=(",", ":")).encode("utf-8")
        text += b" " * (-len(text) % 4)
        data = bytes(self.bin) + b"\0" * (-len(self.bin) % 4)
        total = 12 + 8 + len(text) + (8 + len(data) if data else 0)
        out = bytearray(b"glTF" + struct.pack("<II", 2, total))
        out += struct.pack("<I", len(text)) + b"JSON" + text
        if data:
            out += struct.pack("<I", len(data)) + b"BIN\x00" + data
        return bytes(out)

    def patch_bin(self):
        """Copies weights written in place (into the original file bytes) into the BIN kept here."""
        start = self.glb.bin_at
        if start is not None:
            old = len(self.glb.data) - start
            span = min(old, len(self.bin))
            self.bin[:span] = self.glb.data[start:start + span]


def point_at(line, along, s):
    k = int(np.clip(np.searchsorted(along, s, side="right") - 1, 0, len(line) - 2))
    span = max(along[k + 1] - along[k], 1e-12)
    f = np.clip((s - along[k]) / span, 0.0, 1.0)
    return line[k] + f * (line[k + 1] - line[k])


def default_joints(length, h, small=False):
    low, high = SMALL_JOINTS if small else (JOINTS_MIN, JOINTS_MAX)
    spacing = JOINT_SPACING * h * (0.25 if small else 1.0)
    return int(np.clip(round(length / spacing), low, high))


def chain_weights(part, count, h, mesh, outside):
    """The chain's share of each member's weight and the chain bone each leans on: bone k's
    middle holds it whole, between two middles it fades from one to the next, and near where
    the part hangs from (and where it touches the rest) it fades back into the old weights."""
    along = arc(part["line"])
    total = along[-1]
    s = np.interp(part["t"], part["line_t"], along)
    step = total / count
    x = s / step - 0.5
    lo = np.clip(np.floor(x), 0, count - 1).astype(np.int64)
    f = smoothstep(np.clip(x - lo, 0.0, 1.0))
    f = np.where(x <= 0, 0.0, f)
    hi = np.minimum(lo + 1, count - 1)
    blend = smoothstep(s / (RAMP * step))
    near = outside[mesh.of[part["members"]]]
    band = min(BAND * h, SMALL_BAND * total) if part.get("small") else BAND * h
    blend *= np.where(np.isfinite(near), smoothstep(near / band), 1.0)
    return lo, hi, f, blend, s


def build_chains(glb, skin, body, parts, wanted):
    """Adds each wanted part's chain to the GLB and weights the part to it."""
    h = body.height
    grown = Grown(glb)
    doc = grown.json
    worlds, _ = node_worlds(doc)
    worlds = [np.array(w) for w in worlds]
    skin_index = skin.index
    skin_json = doc["skins"][skin_index]
    mesh = Welded(skin.pos, skin.tris, 1e-5 * h)
    member = np.zeros(len(skin.pos), bool)
    for part, _ in wanted:
        member[part["members"]] = True
    # Surface distance from what stays as it was, so a part touching the rest does not tear.
    seed = np.zeros(mesh.n, bool)
    seed[mesh.of[~member]] = True
    outside = surface_distance(mesh.n, mesh.edges, mesh.length, seed, BAND * h)
    joints = skin.joints.copy()
    weights = skin.weights.copy()
    extra_j = np.zeros((len(skin.pos), 2), np.int64)
    extra_w = np.zeros((len(skin.pos), 2))
    ibm = list(skin.ibm)
    joint_nodes = list(skin_json["joints"])
    chains = []
    for part, spec in wanted:
        count = int(spec.get("joints") or default_joints(part["length"], h, part.get("small")))
        count = int(np.clip(count, 1, JOINTS_MAX * 2))
        root_node = skin.joint_nodes[body.joint[part["root"]]]
        line = part["line"]
        along = arc(line)
        heads = [point_at(line, along, along[-1] * k / count) for k in range(count + 1)]
        names, first = [], len(joint_nodes)
        parent, parent_world = root_node, worlds[root_node]
        for k, at in enumerate(heads):
            local = np.linalg.inv(parent_world) @ np.append(at, 1.0)
            name = f"{SPRING_PREFIX}{spec['id']}_{k}"
            node = grown.node(name, local[:3], parent)
            world = parent_world @ translation(local[:3])
            worlds.append(world)
            joint_nodes.append(node)
            ibm.append(np.linalg.inv(world))
            names.append(name)
            parent, parent_world = node, world
        lo, hi, f, blend, s = chain_weights(part, count, h, mesh, outside)
        m = part["members"]
        joints[m] = joints[m]
        weights[m] *= (1.0 - blend)[:, None]
        extra_j[m, 0] = first + lo
        extra_j[m, 1] = first + hi
        extra_w[m, 0] = blend * (1.0 - f)
        extra_w[m, 1] = blend * f
        chains.append({
            "id": spec["id"], "kind": part["kind"], "rootBone": part["root"],
            "rootJoint": skin.names[body.joint[part["root"]]], "joints": names,
            "length": round(float(along[-1]), 4), "vertices": int(len(m)),
            "blended": int(((blend > 0.0) & (blend < 1.0)).sum()),
            "heads": [[round(float(c), 5) for c in p] for p in heads],
            "radius": round(float(np.median(np.linalg.norm(
                skin.pos[m] - np.array([point_at(line, along, x) for x in s]), axis=1))), 4),
        })
    all_j = np.concatenate([joints, extra_j], 1)
    all_w = np.concatenate([weights, extra_w], 1)
    out_j, out_w = top_four(all_j, all_w)
    if len(joint_nodes) > 65535:
        raise Refused("the chains would pass the 65535 joints a skin can hold")
    skin_json["joints"] = joint_nodes
    mats = np.array([m.T.reshape(16) for m in ibm])
    skin_json["inverseBindMatrices"] = grown.accessor(mats, 5126, "MAT4")
    store_weights(glb, grown, skin, out_j, out_w, member, len(joint_nodes))
    return grown, chains, out_j, out_w


def translation(t):
    m = np.eye(4)
    m[:3, 3] = t
    return m


def store_weights(glb, grown, skin, joints, weights, rows, joint_count):
    """Writes the new weights of the rows that changed. WEIGHTS_0 is written in place; JOINTS_0
    too when the new joints fit its component type, else as a new accessor of 16 bit joints."""
    narrow = joint_count > 256
    for prim, sets, start, count in skin.prims:
        at = prim["attributes"]
        sl = slice(start, start + count)
        if not rows[sl].any():
            continue
        ja, wa = at["JOINTS_0"], at["WEIGHTS_0"]
        old_j = glb.read(ja)
        old_w = glb.read(wa)
        mask = rows[sl]
        old_j[mask] = joints[sl][mask]
        old_w[mask] = weights[sl][mask]
        small = glb.json["accessors"][ja]["componentType"] == 5121
        if small and narrow:
            at["JOINTS_0"] = grown.accessor(old_j.astype(np.uint16), 5123, "VEC4")
        else:
            glb.write(ja, old_j)
        glb.write(wa, old_w)
        if 1 in sets:
            for key in ("JOINTS_1", "WEIGHTS_1"):
                old = glb.read(at[key])
                old[mask] = 0.0
                glb.write(at[key], old)
    grown.patch_bin()


# Colliders, sized from the measured body.

# Each collider: its name, the bone it sits on, and the bones its capsule runs between (a sphere
# when the second is None), and the body segment whose radius sizes it.
COLLIDERS = [
    ("Head", "head", ("head", None), "head>top"),
    ("Neck", "neck", ("neck", "head"), "neck>head"),
    ("Chest", "upperChest", ("upperChest", "neck"), "chest>upperChest"),
    ("Chest", "chest", ("chest", "upperChest"), "spine>chest"),
    ("Hips", "hips", ("hips", "spine"), "hips>spine"),
    ("Arms", "leftUpperArm", ("leftUpperArm", "leftLowerArm"), "leftUpperArm>leftLowerArm"),
    ("Arms", "rightUpperArm", ("rightUpperArm", "rightLowerArm"),
     "rightUpperArm>rightLowerArm"),
    ("Legs", "leftUpperLeg", ("leftUpperLeg", "leftLowerLeg"), "leftUpperLeg>leftLowerLeg"),
    ("Legs", "rightUpperLeg", ("rightUpperLeg", "rightLowerLeg"),
     "rightUpperLeg>rightLowerLeg"),
]
# The collider groups each kind of spring touches.
GROUPS = {
    "hair": ["Head", "Neck", "Chest", "Arms"],
    "cloth": ["Chest", "Arms", "Hips", "Legs"],
    "accessory": ["Chest", "Hips"],
}


def colliders(body, skin, worlds):
    """Sphere and capsule colliders on the head, neck, chest, hips, upper arms and upper legs,
    as VRM wants them: on a bone, with the offset (and a capsule's tail) in that bone's space,
    and the radius the body measured there."""
    h = body.height
    out = []
    for group, bone, (a, b), segment in COLLIDERS:
        if bone not in body.joint or a not in body.joint:
            continue
        k = body.names.index(segment) if segment in body.names else None
        radius = float(body.radius[k].mean()) if k is not None else 0.05 * h
        node = skin.joint_nodes[body.joint[bone]]
        inv = np.linalg.inv(worlds[node])
        start = body.at(a)
        if b is None:
            centre = start + np.array([0.0, 0.5 * HEAD_TOP * h, 0.0])
            shape = {"sphere": {"offset": local_point(inv, centre),
                                "radius": round(radius * 1.15, 4)}}
        else:
            if b not in body.joint:
                continue
            end = body.at(b)
            # A capsule stops a radius short of the next bone, so neighbours overlap less.
            span = np.linalg.norm(end - start)
            tail = start + (end - start) * max(span - 0.5 * radius, 0.0) / max(span, 1e-9)
            shape = {"capsule": {"offset": local_point(inv, start),
                                 "radius": round(radius, 4),
                                 "tail": local_point(inv, tail)}}
        out.append({"group": group, "bone": bone, "joint": skin.names[body.joint[bone]],
                    "shape": shape})
    return out


def local_point(inv, p):
    q = inv @ np.append(p, 1.0)
    return [round(float(x), 5) for x in q[:3]]


def groups_for(kind, chain, body, present):
    names = list(GROUPS.get(kind, GROUPS["accessory"]))
    # Hair or a charm long enough to reach the hips and legs touches them too.
    low = min(p[1] for p in chain["heads"])
    if low < body.at("hips")[1] + 0.05 * body.height:
        for extra in ("Hips", "Legs"):
            if extra not in names:
                names.append(extra)
    return [n for n in names if n in present]


# Sheets.


def hex_rgb(code):
    return tuple(int(code[i:i + 2], 16) / 255.0 for i in (1, 3, 5))


class Stage:
    """A headless Blender scene with the model, for sheets: Blender's axes are glTF's with Y up
    turned to Z up, so the model faces the front camera at -Y."""

    def __init__(self, path, height):
        import bpy

        self.bpy = bpy
        bpy.ops.wm.read_factory_settings(use_empty=True)
        bpy.ops.import_scene.gltf(filepath=path)
        for obj in list(bpy.data.objects):
            if any(c.name == "glTF_not_exported" for c in obj.users_collection):
                bpy.data.objects.remove(obj)
        arms = [o for o in bpy.data.objects if o.type == "ARMATURE"]
        self.arm = arms[0] if arms else None
        self.height = height
        sc = bpy.context.scene
        sc.render.engine = "BLENDER_WORKBENCH"
        sh = sc.display.shading
        sh.light = "STUDIO"
        sh.color_type = "TEXTURE"
        sh.show_backface_culling = False
        sh.background_type = "VIEWPORT"
        sh.background_color = (0.55, 0.58, 0.62)
        sc.render.resolution_x = sc.render.resolution_y = CELL
        sc.render.resolution_percentage = 100
        sc.render.film_transparent = False
        sc.render.image_settings.file_format = "PNG"
        sc.view_settings.view_transform = "Standard"
        cam = bpy.data.objects.new("sheet_cam", bpy.data.cameras.new("sheet_cam"))
        sc.collection.objects.link(cam)
        cam.data.type = "ORTHO"
        cam.data.clip_end = 30.0
        sc.camera = cam
        label = bpy.data.objects.new("sheet_label", bpy.data.curves.new("sheet_label", "FONT"))
        sc.collection.objects.link(label)
        self.cam, self.label = cam, label
        if self.arm is not None:
            for pb in self.arm.pose.bones:
                pb.matrix_basis.identity()
            if self.arm.animation_data:
                self.arm.animation_data.action = None
        self.update()

    def update(self):
        self.bpy.context.view_layer.update()

    def material(self, name, colour):
        mat = self.bpy.data.materials.new(name)
        mat.diffuse_color = (*colour, 1.0)
        return mat

    def overlay(self, name, pos, tris, normals, colour, lift):
        """A coloured copy of some triangles, lifted off the surface so it shows over it."""
        used = np.unique(tris)
        remap = np.full(len(pos), -1)
        remap[used] = np.arange(len(used))
        p = pos[used] + normals[used] * lift
        p = np.stack([p[:, 0], -p[:, 2], p[:, 1]], 1)
        me = self.bpy.data.meshes.new(name)
        me.from_pydata(p.tolist(), [], remap[tris].tolist())
        me.materials.append(self.material(name, colour))
        obj = self.bpy.data.objects.new(name, me)
        self.bpy.context.scene.collection.objects.link(obj)
        return obj

    def marker(self, name, at, radius, colour):
        import bmesh

        me = self.bpy.data.meshes.new(name)
        bm = bmesh.new()
        bmesh.ops.create_uvsphere(bm, u_segments=12, v_segments=8, radius=radius)
        bm.to_mesh(me)
        bm.free()
        me.materials.append(self.material(name, colour))
        obj = self.bpy.data.objects.new(name, me)
        obj.location = (at[0], -at[2], at[1])
        self.bpy.context.scene.collection.objects.link(obj)
        return obj

    def swing(self, chains, direction, degrees):
        """Bends every chain toward `direction` (glTF axes): each bone turns the same share of
        `degrees` about its head, so the chain curls smoothly."""
        from mathutils import Matrix, Vector

        if self.arm is None:
            return
        target = Vector((direction[0], -direction[2], direction[1]))
        mw = self.arm.matrix_world
        for chain in chains:
            names = [n for n in chain["joints"][:-1] if n in self.arm.pose.bones]
            if not names:
                continue
            step = math.radians(degrees) / len(names)
            for name in names:
                pb = self.arm.pose.bones[name]
                tail = self.arm.pose.bones.get(chain["joints"][chain["joints"].index(name) + 1])
                head = mw @ pb.head
                down = ((mw @ tail.head) - head).normalized() if tail else Vector((0, 0, -1))
                axis = down.cross(target)
                if axis.length < 1e-6:
                    continue
                rot = Matrix.Rotation(step, 4, axis.normalized())
                world = mw @ pb.matrix
                world = Matrix.Translation(head) @ rot @ Matrix.Translation(-head) @ world
                pb.matrix = mw.inverted() @ world
                self.update()

    def rest(self):
        if self.arm is not None:
            for pb in self.arm.pose.bones:
                pb.matrix_basis.identity()
            self.update()

    def place(self, view, focus, text):
        from mathutils import Vector

        h = self.height
        if view in ("close", "near"):
            # A near look frames a small piece; a close one where a long part hangs from.
            scale = (0.42 if view == "close" else NEAR_LOOK) * h
            centre = Vector((focus[0], -focus[2], focus[1]))
            # Look at where the part hangs from, from the side it sits on, a little from above.
            yaw = math.atan2(focus[0], focus[2]) if abs(focus[0]) + abs(focus[2]) > 1e-6 else 0.0
            d = Vector((math.sin(yaw), -math.cos(yaw), 0.15)).normalized()
        else:
            scale, centre = 1.1 * h, Vector((0.0, 0.0, 0.5 * h))
            yaw = math.radians({"front": 0.0, "side": 90.0, "back": 180.0}[view])
            d = Vector((math.sin(yaw), -math.cos(yaw), 0.0))
        cam = self.cam
        cam.data.ortho_scale = scale
        cam.location = centre + d * 10.0
        cam.rotation_euler = (-d).to_track_quat("-Z", "Y").to_euler()
        flat = Vector((d.x, d.y, 0.0))
        flat = flat.normalized() if flat.length > 1e-6 else Vector((0.0, -1.0, 0.0))
        right = Vector((-flat.y, flat.x, 0.0))
        label = self.label
        label.data.body = text
        label.data.size = 0.03 * scale
        label.location = centre + d * 5.0 - right * 0.46 * scale + Vector((0, 0, 0.44 * scale))
        label.rotation_euler = d.to_track_quat("Z", "Y").to_euler()

    def shoot(self, out, rows, title):
        """Renders one cell per (row, view) and joins them into one sheet. `rows` is a list of
        (name, pose function) and each cell's view and focus come from `views`."""
        bpy = self.bpy
        cells = out + "_cells"
        os.makedirs(cells, exist_ok=True)
        width = max(len(views) for _, _, views in rows)
        sheet = np.ones((CELL * len(rows), CELL * width, 4), np.float32)
        for ri, (name, pose, views) in enumerate(rows):
            self.rest()
            pose()
            for ci, (view, focus) in enumerate(views):
                self.place(view, focus, f"{title}  {name}  {view}")
                cell = os.path.join(cells, f"{ri}_{ci}.png")
                bpy.context.scene.render.filepath = cell
                bpy.ops.render.render(write_still=True)
                img = bpy.data.images.load(cell)
                px = np.array(img.pixels[:], np.float32).reshape(CELL, CELL, 4)[::-1]
                sheet[ri * CELL:(ri + 1) * CELL, ci * CELL:(ci + 1) * CELL] = px
                bpy.data.images.remove(img)
                os.remove(cell)
                if ci:
                    sheet[ri * CELL:(ri + 1) * CELL, ci * CELL:ci * CELL + 2, :3] = 0.15
            if ri:
                sheet[ri * CELL:ri * CELL + 2, :, :3] = 0.15
        os.rmdir(cells)
        self.rest()
        img = bpy.data.images.new("sheet", sheet.shape[1], sheet.shape[0], alpha=True)
        img.pixels.foreach_set(sheet[::-1].ravel())
        img.filepath_raw = out
        img.file_format = "PNG"
        img.save()
        bpy.data.images.remove(img)
        log(f"sheet {out}")


def vertex_normals(pos, tris):
    n = np.zeros_like(pos)
    face = np.cross(pos[tris[:, 1]] - pos[tris[:, 0]], pos[tris[:, 2]] - pos[tris[:, 0]])
    for k in range(3):
        np.add.at(n, tris[:, k], face)
    length = np.linalg.norm(n, axis=1, keepdims=True)
    return n / np.maximum(length, 1e-12)


def paint(stage, skin, parts, colours):
    """Overlays each part's triangles in its colour."""
    normals = vertex_normals(skin.pos, skin.tris)
    for k, (part, colour) in enumerate(zip(parts, colours)):
        inside = np.zeros(len(skin.pos), bool)
        inside[part["members"]] = True
        tris = skin.tris[inside[skin.tris].all(1)]
        if len(tris):
            stage.overlay(f"part_{k}", skin.pos, tris, normals, hex_rgb(colour),
                          0.002 * stage.height)


def candidates_sheet(path, out, skin, parts, height):
    stage = Stage(path, height)
    paint(stage, skin, parts, [p["color"] for p in parts])
    views = [("front", None), ("side", None), ("back", None)]
    rows = [("found", lambda: None, views)]
    small = [p for p in parts if p.get("small")]
    if small:
        # A close look at the first few small pieces, where each hangs from.
        rows.append(("small pieces", lambda: None, [("near", p["line"][0]) for p in small[:3]]))
    stage.shoot(out, rows, "dangling parts")


def chain_sheets(job, skin, wanted, chains, height):
    parts = [part for part, _ in wanted]
    # The close look sits a little down the first part, where it leaves the root.
    focus = np.zeros(3)
    if parts:
        line = parts[0]["line"]
        along = arc(line)
        focus = point_at(line, along, 0.15 * along[-1])
    views = [("front", None), ("side", None), ("back", None), ("close", focus)]
    small = [part for part in parts if part.get("small")]
    if small:
        line = small[0]["line"]
        views.append(("near", point_at(line, arc(line), 0.5 * arc(line)[-1])))
    if "before" in job["sheets"]:
        stage = Stage(job["input"], height)
        paint(stage, skin, parts, [p["color"] for p in parts])
        for chain in chains:
            for k, at in enumerate(chain["heads"]):
                stage.marker(f"{chain['id']}_{k}", at, 0.008 * height, (1.0, 1.0, 1.0))
        stage.shoot(job["sheets"]["before"], [("chains", lambda: None, views)], "before")
    if "after" in job["sheets"]:
        stage = Stage(job["output"], height)
        rows = [("rest", lambda: None, views)]
        for name, direction in SWAYS:
            rows.append((name, lambda d=direction: stage.swing(chains, d, SWAY_DEG), views))
        stage.shoot(job["sheets"]["after"], rows, "after")


# The two modes.


def rounded(points):
    return [[round(float(c), 4) for c in p] for p in points]


def candidates(parts, h):
    """The parts as candidates, numbered per kind, most sure first."""
    out = []
    counts = {}
    for k, part in enumerate(parts):
        name = "charm" if part.get("small") else part["kind"]
        counts[name] = counts.get(name, 0) + 1
        part["id"] = f"{name}-{counts[name]}"
        part["color"] = COLOURS[k % len(COLOURS)]
        out.append({
            "id": part["id"],
            "kind": part["kind"],
            "rootBone": part["root"],
            "line": rounded(part["line"]),
            "length": round(part["length"], 4),
            "drop": round(part["drop"], 4),
            "vertices": int(len(part["members"])),
            "confidence": part["confidence"],
            "joints": default_joints(part["length"], h, part.get("small")),
            "color": part["color"],
            "small": bool(part.get("small")),
        })
    return out


def line_gap(a, b):
    """How far, on average, the points of line `a` lie from line `b`."""
    a, b = np.asarray(a, np.float64), np.asarray(b, np.float64)
    if len(a) == 0 or len(b) < 2:
        return np.inf
    return float(np.mean([min(segment_distance(p[None], b[i], b[i + 1])[0]
                              for i in range(len(b) - 1)) for p in a]))


def match(parts, springs, h):
    """Each confirmed spring with the part it names: same root, its line on the part's."""
    wanted = []
    for spec in springs:
        best, gap = None, np.inf
        for part in parts:
            if part["root"] != spec.get("rootBone"):
                continue
            g = line_gap(spec.get("line", []), part["line"])
            if g < gap:
                best, gap = part, g
        if best is None or gap > MATCH * h:
            raise Refused(f"the spring {spec.get('id')} no longer matches a part hanging off "
                          "this model; look for dangling parts again")
        if any(best is p for p, _ in wanted):
            raise Refused(f"the spring {spec.get('id')} names a part another spring has")
        wanted.append((best, spec))
    return wanted


def find_mode(job, data):
    glb = Glb(bytes(data))
    skin = main_skin(glb)
    body, parts = find_parts(skin, job["bones"])
    found = candidates(parts, body.height)
    with open(job["candidates"], "w", encoding="utf-8") as fh:
        json.dump({"height": round(body.height, 4), "vertices": int(len(skin.pos)),
                   "candidates": found}, fh, indent=2)
    log(f"{len(found)} dangling parts: " + ", ".join(
        f"{c['id']} from {c['rootBone']} ({c['confidence']:.2f})" for c in found))
    if "candidates" in job["sheets"]:
        candidates_sheet(job["input"], job["sheets"]["candidates"], skin, parts, body.height)


def main_skin(glb):
    skins = skins_of(glb)
    if not skins:
        raise Refused("no mesh is skinned")
    # The skin with the most vertices carries the body.
    return max(skins, key=lambda s: len(s.pos))


def chain_mode(job, data):
    glb = Glb(bytes(data))
    skin = main_skin(glb)
    body, parts = find_parts(skin, job["bones"])
    candidates(parts, body.height)
    springs = job.get("springs", [])
    if not springs:
        raise Refused("no spring is confirmed")
    wanted = match(parts, springs, body.height)
    for k, (part, spec) in enumerate(wanted):
        part["color"] = COLOURS[k % len(COLOURS)]
        part["id"] = spec["id"]
    grown, chains, joints, weights = build_chains(glb, skin, body, parts, wanted)
    worlds, _ = node_worlds(grown.json)
    found = colliders(body, skin, worlds)
    present = {c["group"] for c in found}
    for chain in chains:
        chain["colliderGroups"] = groups_for(chain["kind"], chain, body, present)
    report = {
        "height": round(body.height, 4),
        "chains": chains,
        "colliders": found,
        "weights": weight_checks(joints, weights),
        "changed": int(sum(len(p["members"]) for p, _ in wanted)),
    }
    with open(job["output"], "wb") as fh:
        fh.write(grown.bytes())
    log(f"{len(chains)} chains, {len(found)} colliders, "
        f"{report['changed']} vertices weighted to the chains")
    if job["sheets"]:
        chain_sheets(job, skin, wanted, chains, body.height)
    with open(job["report"], "w", encoding="utf-8") as fh:
        json.dump(report, fh, indent=2)
    log(f"wrote {job['output']}")


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:]
    parser = argparse.ArgumentParser()
    parser.add_argument("--job", required=True)
    return parser.parse_args(argv)


def load_job(path):
    with open(path, encoding="utf-8") as fh:
        job = json.load(fh)
    base = os.path.dirname(os.path.abspath(path))

    def resolve(p):
        return p if os.path.isabs(p) else os.path.join(base, p)

    for key in ("input", "output", "report", "candidates"):
        if key in job:
            job[key] = resolve(job[key])
    job["sheets"] = {k: resolve(v) for k, v in job.get("sheets", {}).items()}
    return job


def main():
    job = load_job(parse_args().job)
    with open(job["input"], "rb") as fh:
        data = fh.read()
    mode = job.get("mode", "find")
    if job["sheets"]:
        try:
            import bpy  # noqa: F401
        except ImportError:
            fail("the sheets need Blender; run this with blender -b")
    try:
        if mode == "find":
            find_mode(job, data)
        elif mode == "chain":
            chain_mode(job, data)
        else:
            fail(f"unknown mode {mode}; use find or chain")
    except Refused as e:
        fail(str(e))


if __name__ == "__main__":
    try:
        main()
    except SystemExit:
        raise
    except Exception:
        traceback.print_exc()
        fail("finding springs stopped: " + traceback.format_exc().strip().splitlines()[-1])
