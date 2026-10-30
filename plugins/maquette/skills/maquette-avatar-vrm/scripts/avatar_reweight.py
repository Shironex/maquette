"""Free cloth and hair from the arms of a rigged character before its arms are lifted.

Auto rigs often weight a cape, a long coat, hair or a wing of cloth beside the arms to the arm
bones, so lifting the arms to a T-pose drags them along or tears them. This moves that weight to
the trunk and leaves the arm itself (skin, sleeves, gloves) as it was.

Maquette runs this headless, never with a window, on a copy of the model:

    blender -b --factory-startup --python-exit-code 1 --python avatar_reweight.py -- \
        --job job.json

job.json names the model, where the outputs go and the VRM bone roles, each a VRM humanoid bone
name to the joint (node) name it maps to, as the VRM readiness found them. Relative paths resolve
against the job file's folder:

    {"input": "/abs/model.glb", "output": "reweighted.glb", "report": "report.json",
     "sheets": {"before": "sheet_before.png", "after": "sheet_after.png"},
     "bones": {"hips": "mixamorig:Hips", "leftUpperArm": "mixamorig:LeftArm", ...},
     "bridges": "hide"}

Only the bytes of the JOINTS_0 and WEIGHTS_0 accessors change (JOINTS_1 and WEIGHTS_1 are
cleared when present): the mesh, UVs, textures, bone names and the rest pose stay byte for byte,
unless bridges are hidden (below).
Everything is read in each skin's bind space (vertex positions as stored, each joint where its
inverse bind matrix puts it) and measured against the model's own height, so the scale of the
export does not matter. The arm chain of a side is its upper arm joint and every joint below it.

Which vertices belong to the arm, per side:

1. Cross sections: the welded mesh (UV seams joined) is cut by planes square to the arm every
   1% of the height, from the shoulder to the finger tips, and each cut is joined into curves
   through the mesh edges they share. A sleeve, a glove or bare skin is a closed curve around
   the bone; a cape or a strand of hair beside the arm is not.
2. The limb in each cut is the curve that holds the bone (or passes very near it), followed from
   cut to cut where its radius and middle carry on smoothly, so a sleeve keeps being found where
   the bone pokes through it. Where a cape is sewn to the back of a sleeve the cut is one long
   loop: its middle is the deepest point inside it near the bone, the limb radius is how far that
   point is from the loop, and only the part of the loop within a few radii of it is the limb.
3. Those limb curves seed the arm and the other curves in the cuts seed the rest; both spread
   over the surface near the arm (geodesic, along mesh edges) and the nearer seed wins. Where no
   cut found the limb, the vertices within a little more than the limb radius of its middle are
   the arm. A piece of its own that no cut gave to the rest (a separate sleeve or glove, a
   finger) goes with the arm whole, and so does a small island lying mostly on the arm (a
   cuff, a bracelet).
4. The hand always stays with the arm, whatever the cuts said, so a nail or a finger tip never
   stays behind when the arm lifts. The hand is its own surface: what the mesh edges reach from
   the arm found just before the wrist, going only through the surface past the wrist near the
   bones (fingers, nails, a glove, a ring sewn on). Hair or cloth that only hangs beside the hand
   joins it through the head or the body, so it is never reached, whatever its weights. A small
   piece of its own goes with the hand whole when most of it lies on the hand's surface (a ring)
   or most of it is weighted to the hand and finger bones (a prop the hand holds).

Everything else on the arm chain moves its arm weight to the trunk, shared by inverse distance
between spine, chest and upperChest, and the shoulder for vertices above the armpit (the top of a
cape). At the boundary a smooth band in surface distance (geodesic, over the welded mesh) blends:
just outside the arm a vertex keeps most of its arm weight and the share fades to none, so cloth
sewn to a sleeve stretches instead of tearing. The band only blends where lifting the arm level
moves the surface little, near the shoulder. Further down (a sleeve that rested on the hip, with
faces fused between the two), a share kept on the body would stretch into a sheet as wide as the
band, so there each band vertex goes wholly to the arm or wholly to the body, split along the
seam that costs least to stretch: a minimum cut over the mesh edges, each costing its length
times how far the lift moves it. Where the lift moves it far, the arm's own surface follows the
arm wholly too, giving its arm bones the weight an auto rig put on the body or the legs. The
trunk shares what it takes over its two nearest bones, so it never crowds a kept arm weight out
of the four influences. At most 4 influences per vertex are kept and they sum to 1; no vertex is
left without weight.

Faces that join a part following the arm to a part staying with the body still stretch when the
arm lifts, whatever the weights: those bridges are counted and named in the report. With
"bridges": "hide" (the default) their triangles are taken out of the index buffer, so they are
not drawn: the kept indices move to the front of the accessor and its count shrinks, and every
vertex, UV, texture and weight stays where it was. A primitive whose indices another primitive
shares, or that has none or draws strips or fans, keeps its faces. "keep" leaves every face.

The report holds the counts: vertices changed, the weight moved, from which arm bones and to
which trunk bones, the arm vertices kept as sleeve, the band, the arm vertices that let go of
body weight, the bridges (side, faces, middle and stretched length), the bridge faces hidden,
and the limb radii. The two sheets
show the model before and after, turned to face the camera, in rows rest, arms level (the
T-pose) and arms raised, and columns front, side, back and a close look at the shoulders.
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
SIDES = ("left", "right")

# Tunables, in shares of the model's height unless said otherwise.
PLANE = 0.01         # spacing of the cuts across the arm
NEAR = 0.15          # surface further than this from the arm is never read
LIMB_MIN, LIMB_MAX = 0.004, 0.06   # radius of a limb, inside its loop
LIMB_REACH = 0.12    # loops wider than this are the body, not a limb
HUG = 0.35           # a loop passing this many limb radii from the bone still holds it
DRIFT = 0.3          # how much nearer the bone the limb's middle is preferred
ROUND = 1.8          # the limb is its loop within this many radii of its middle
FINGER = 0.03        # in the hand, closed loops this small may be fingers
CARRY = 1.6          # a loop carries the limb on when its radius is within this factor...
STEER = 0.6          # ...and its middle within this many radii of the last one
CORE_Q = 1.3         # limb radii around the limb's middle that count as the arm itself
ISLAND_IN = 0.7      # share of an island inside the tube that makes it a cuff
ISLAND_MAX = 0.02    # ...when it is at most this share of the skinned vertices
BAND = 0.1           # width of the smooth falloff, surface distance
SPLIT_LIFT = 0.04    # past this lift, the band splits between arm and body instead of blending
FOLLOW_LIFT = 0.08   # the arm's own surface follows it wholly where the lift moves it twice this
STRETCHED = 2.5      # a face whose edge grows past this many times its length when lifted
BRIDGE_FROM = 0.1    # stretched faces this far from the shoulder...
BRIDGE_LONG = 0.1    # ...stretched longer than this are a bridge the lift cannot avoid
BRIDGES_LISTED = 8   # the largest bridges the report names
TRUNK_BONES = 2      # trunk bones sharing what an arm lets go of, per vertex
ARMPIT = 0.04        # the shoulder takes cloth above the armpit, fading over this
IDW_POWER = 4.0
CHANGED = 1e-3       # arm weight a vertex must lose to count as moved
HAND_REACH = 0.06    # the hand's surface lies past the wrist within this of the bones...
HAND_WRIST = 0.03    # ...reached from the arm found up to this far before the wrist
HAND_TOUCH = 0.01    # a piece this near the hand's surface lies on it
HAND_HELD = 0.5      # a vertex with more than this share of its weight on the hand is held by it


def log(text):
    print(f"{TAG} {text}", flush=True)


def fail(text):
    print(f"{TAG} Error: {text}", flush=True)
    sys.exit(1)


class Refused(Exception):
    """A model this script cannot reweight safely; the message says why."""


# glTF container: read the accessors, patch the skin weights in place.

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


    def shrink(self, index, values):
        """Writes `values` (unsigned integers) over the start of an index accessor and makes it
        that many long; the bytes past them stay as they were and nothing else moves."""
        acc, dtype, width, offset, stride = self.view(index)
        if width != 1 or stride != dtype.itemsize or len(values) > acc["count"]:
            raise Refused("an index accessor is not a plain list")
        out = np.ndarray((len(values),), dtype, self.data, offset)
        out[...] = values.astype(dtype)
        acc["count"] = int(len(values))
        if "min" in acc:
            acc["min"] = [int(values.min())] if len(values) else [0]
        if "max" in acc:
            acc["max"] = [int(values.max())] if len(values) else [0]

    def bytes(self):
        """The file with this JSON, and the binary chunk as it is."""
        text = json.dumps(self.json, separators=(",", ":")).encode()
        text += b" " * (-len(text) % 4)
        old = struct.unpack("<I", self.data[12:16])[0]
        rest = bytes(self.data[20 + old:])
        head = struct.pack("<4sII", b"glTF", 2, 20 + len(text) + len(rest))
        return head + struct.pack("<I4s", len(text), b"JSON") + text + rest


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
        if "inverseBindMatrices" in skin:
            ibm = glb.read(skin["inverseBindMatrices"]).reshape(-1, 4, 4).transpose(0, 2, 1)
            bind = np.linalg.inv(ibm)
        else:
            bind = np.array([worlds[n] for n in self.joint_nodes])
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
        # Per primitive, where its triangles start in `tris` and how many there are.
        self.tri_ranges = []
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
            self.tri_ranges.append((sum(len(x) for x in tris), len(t)))
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

    def descendants(self, root):
        out = {root}
        grew = True
        while grew:
            grew = False
            for i, p in enumerate(self.parent):
                if p in out and i not in out:
                    out.add(i)
                    grew = True
        return out

    def store(self, glb, joints, weights, rows):
        """Writes new weights for the rows that changed, back into each primitive's accessors."""
        for prim, sets, start, count in self.prims:
            at = prim["attributes"]
            sl = slice(start, start + count)
            if not rows[sl].any():
                continue
            for k in sets:
                ja, wa = at[f"JOINTS_{k}"], at[f"WEIGHTS_{k}"]
                old_j = glb.read(ja)
                old_w = glb.read(wa)
                new_j = np.zeros_like(old_j) if k else joints[sl].astype(np.float64)
                new_w = np.zeros_like(old_w) if k else weights[sl]
                mask = rows[sl]
                old_j[mask] = new_j[mask]
                old_w[mask] = new_w[mask]
                glb.write(ja, old_j)
                glb.write(wa, old_w)


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


def hide_faces(glb, skin, faces):
    """Takes the skin's triangles `faces` (indices into skin.tris) out of their primitives' index
    buffers. Returns how many were hidden; a primitive drawn without indices, as strips or fans,
    or whose indices another primitive shares keeps its faces."""
    shared = {}
    for mesh in glb.json.get("meshes", []):
        for prim in mesh["primitives"]:
            if "indices" in prim:
                shared[prim["indices"]] = shared.get(prim["indices"], 0) + 1
    faces = np.unique(np.asarray(faces, np.int64))
    hidden = 0
    for (prim, _, start, _), (first, count) in zip(skin.prims, skin.tri_ranges):
        mine = faces[(faces >= first) & (faces < first + count)] - first
        if len(mine) == 0 or prim.get("mode", 4) != 4 or "indices" not in prim:
            continue
        if shared[prim["indices"]] > 1:
            continue
        idx = glb.read(prim["indices"]).astype(np.int64).ravel()
        tris = idx[: len(idx) // 3 * 3].reshape(-1, 3)
        keep = np.ones(len(tris), bool)
        keep[mine] = False
        glb.shrink(prim["indices"], tris[keep].ravel())
        hidden += int(len(mine))
    return hidden


# Geometry helpers.


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
        whole = (t[:, 0] != t[:, 1]) & (t[:, 1] != t[:, 2]) & (t[:, 2] != t[:, 0])
        # Each welded face's triangle in the skin's own list.
        self.source = np.nonzero(whole)[0]
        t = t[whole]
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


class Body:
    """Bone roles to joints, and the body's axes in bind space."""

    def __init__(self, skin, bones):
        by_name = {}
        for i, name in enumerate(skin.names):
            by_name.setdefault(name, i)
        self.joint = {role: by_name[name] for role, name in bones.items() if name in by_name}
        for role in ("hips", "spine", "leftUpperArm", "rightUpperArm", "leftLowerArm",
                     "rightLowerArm", "leftHand", "rightHand"):
            if role not in self.joint:
                raise Refused(f"the bone map has no {role} in this skin")
        self.skin = skin
        top = next((r for r in ("head", "neck", "upperChest", "chest") if r in self.joint), None)
        if top is None:
            raise Refused("the bone map has neither head, neck nor chest")
        self.up = unit(self.at(top) - self.at("hips"))
        left = self.at("leftUpperArm") - self.at("rightUpperArm")
        left = unit(left - (left @ self.up) * self.up)
        self.left = left
        self.forward = unit(np.cross(left, self.up))
        heights = skin.pos @ self.up
        self.height = float(heights.max() - heights.min()) if len(heights) else 0.0
        if self.height <= 1e-9:
            raise Refused("the skinned mesh has no height")

    def at(self, role):
        return self.skin.at[self.joint[role]]

    def has(self, role):
        return role in self.joint


class Arm:
    """One side's arm: a polyline from the shoulder joint to the finger tips."""

    def __init__(self, body, side):
        skin = body.skin
        self.side = side
        self.chain = sorted(skin.descendants(body.joint[side + "UpperArm"]))
        u, lo, hd = (body.at(side + b) for b in ("UpperArm", "LowerArm", "Hand"))
        hand_bones = skin.descendants(body.joint[side + "Hand"]) - {body.joint[side + "Hand"]}
        fore = unit(hd - lo)
        reach = 0.0
        for j in hand_bones:
            reach = max(reach, float((skin.at[j] - hd) @ fore))
        if reach < 0.25 * np.linalg.norm(hd - lo) * 0.3:
            reach = 0.35 * float(np.linalg.norm(hd - lo))
        tip = hd + fore * reach
        self.points = np.array([u, lo, hd, tip])
        self.lengths = np.linalg.norm(np.diff(self.points, axis=0), axis=1)
        self.starts = np.concatenate([[0.0], np.cumsum(self.lengths)])
        self.length = float(self.starts[-1])
        self.hand_from = float(self.starts[2])
        self.frames = []
        for a, b in zip(self.points[:-1], self.points[1:]):
            axis = unit(b - a)
            ref = body.forward - (body.forward @ axis) * axis
            if np.linalg.norm(ref) < 1e-6:
                ref = body.up - (body.up @ axis) * axis
            e1 = unit(ref)
            self.frames.append((axis, e1, np.cross(axis, e1)))

    def at(self, s):
        """The point on the bones at arc length `s`."""
        k = int(np.clip(np.searchsorted(self.starts, s, side="right") - 1, 0, len(self.frames) - 1))
        return self.points[k] + self.frames[k][0] * (s - self.starts[k])

    def project(self, pts):
        """Arc length along the arm, the segment, the offset from the bone square to it, and
        whether the point lies above the shoulder end (past the upper arm's head)."""
        best = np.full(len(pts), np.inf)
        s = np.zeros(len(pts))
        seg = np.zeros(len(pts), np.int64)
        radial = np.zeros_like(pts)
        above = np.zeros(len(pts), bool)
        for k, (a, (axis, _, _)) in enumerate(zip(self.points[:-1], self.frames)):
            rel = pts - a
            t_raw = rel @ axis
            t = np.clip(t_raw, 0.0, self.lengths[k])
            off = rel - t[:, None] * axis
            d = np.linalg.norm(off, axis=1)
            take = d < best
            best[take] = d[take]
            s[take] = self.starts[k] + t[take]
            seg[take] = k
            radial[take] = off[take]
            if k == 0:
                above = np.where(take, t_raw < 0.0, above)
            else:
                above &= ~take
        return s, seg, radial, above


def cross_section(arm, s, wpos, wtris, tri_edges, h):
    """Cuts the welded mesh with the plane square to the arm at arc length `s`, and joins the
    cut into curves through the mesh edges they share. Each curve: its triangles, whether it
    closes, its middle (in the plane, from the bone), its radius and reach, and whether the bone
    lies inside it or how near."""
    k = int(np.clip(np.searchsorted(arm.starts, s, side="right") - 1, 0, len(arm.frames) - 1))
    axis, e1, e2 = arm.frames[k]
    origin = arm.points[k] + axis * (s - arm.starts[k])
    rel = wpos - origin
    d = rel @ axis
    radial = np.linalg.norm(rel - d[:, None] * axis, axis=1)
    side = d[wtris] > 0.0
    cut = side.any(1) & ~side.all(1) & (radial[wtris] < NEAR * h).any(1)
    tris = np.nonzero(cut)[0]
    if len(tris) == 0:
        return origin, axis, e1, e2, []
    crossed = np.zeros((len(tris), 3), bool)
    points = np.zeros((len(tris), 3, 3))
    for c in range(3):
        va, vb = wtris[tris, c], wtris[tris, (c + 1) % 3]
        crossed[:, c] = side[tris, c] != side[tris, (c + 1) % 3]
        da, db = d[va], d[vb]
        f = np.where(crossed[:, c], da / np.where(crossed[:, c], da - db, 1.0), 0.0)
        points[:, c] = wpos[va] + (wpos[vb] - wpos[va]) * f[:, None]
    rows = np.arange(len(tris))
    first = np.argmax(crossed, 1)
    second = 2 - np.argmax(crossed[:, ::-1], 1)
    ea, eb = tri_edges[tris, first], tri_edges[tris, second]
    pa, pb = points[rows, first], points[rows, second]
    ids, compact = np.unique(np.concatenate([ea, eb]), return_inverse=True)
    ca, cb = compact[: len(ea)], compact[len(ea):]
    label = components(len(ids), np.stack([ca, cb], 1))
    degree = np.bincount(np.concatenate([ca, cb]), minlength=len(ids))
    curve = label[ca]
    ncurves = int(label.max()) + 1
    open_ends = np.bincount(label, weights=(degree != 2).astype(np.float64), minlength=ncurves)
    flat_a = np.stack([(pa - origin) @ e1, (pa - origin) @ e2], 1)
    flat_b = np.stack([(pb - origin) @ e1, (pb - origin) @ e2], 1)
    mid = (flat_a + flat_b) / 2.0
    length = np.linalg.norm(flat_b - flat_a, axis=1) + 1e-12
    total = np.bincount(curve, weights=length, minlength=ncurves)
    centre = np.stack([np.bincount(curve, weights=mid[:, i] * length, minlength=ncurves)
                       for i in range(2)], 1) / total[:, None]
    away = np.linalg.norm(mid - centre[curve], axis=1)
    radius = np.bincount(curve, weights=away * length, minlength=ncurves) / total
    reach = np.zeros(ncurves)
    np.maximum.at(reach, curve, np.maximum(np.linalg.norm(flat_a - centre[curve], axis=1),
                                           np.linalg.norm(flat_b - centre[curve], axis=1)))
    # Parity of a ray from the bone along +e1, and the nearest the curve comes to the bone.
    ya, yb = flat_a[:, 1], flat_b[:, 1]
    spans = (ya > 0) != (yb > 0)
    t = np.where(spans, ya / np.where(spans, ya - yb, 1.0), 0.0)
    xcut = flat_a[:, 0] + (flat_b[:, 0] - flat_a[:, 0]) * t
    hits = np.bincount(curve, weights=(spans & (xcut > 0)).astype(np.float64), minlength=ncurves)
    seg = flat_b - flat_a
    u = np.clip(-(flat_a * seg).sum(1) / (seg * seg).sum(1).clip(1e-24), 0.0, 1.0)
    gap_seg = np.linalg.norm(flat_a + seg * u[:, None], axis=1)
    gap = np.full(ncurves, np.inf)
    np.minimum.at(gap, curve, gap_seg)
    out = []
    for c in range(ncurves):
        mine = curve == c
        out.append({
            "tris": tris[mine],
            "a": flat_a[mine],
            "b": flat_b[mine],
            "closed": open_ends[c] == 0,
            "centre": centre[c],
            "radius": float(radius[c]),
            "reach": float(reach[c]),
            "inside": int(hits[c]) % 2 == 1,
            "gap": float(gap[c]),
        })
    return origin, axis, e1, e2, out


def segment_gap(points, a, b):
    """Distance from each point to the nearest of the segments a-b (all 2D)."""
    seg = b - a
    rel = points[:, None, :] - a[None, :, :]
    t = np.clip((rel * seg[None]).sum(2) / (seg * seg).sum(1).clip(1e-24)[None], 0.0, 1.0)
    return np.linalg.norm(rel - t[..., None] * seg[None], axis=2).min(1)


def limb_part(curve, h):
    """The part of a cut curve that is the limb: its triangles that are, the limb's middle (in
    the plane, from the bone) and radius, or None when no part is.

    Cloth sewn onto a sleeve joins its loop as a flap; the limb is the round part: around the
    point inside the loop furthest from its walls near the bone, only what lies within a few of
    those radii."""
    if not curve["closed"]:
        return None
    if curve["radius"] > LIMB_REACH * h:
        return None
    a, b = curve["a"], curve["b"]
    lo = np.minimum(a.min(0), b.min(0))
    hi = np.maximum(a.max(0), b.max(0))
    step = max(float((hi - lo).max()) / 40.0, 0.001 * h)
    gx = np.arange(lo[0], hi[0] + step, step)
    gy = np.arange(lo[1], hi[1] + step, step)
    cx, cy = np.meshgrid(gx, gy)
    cand = np.stack([cx.ravel(), cy.ravel()], 1)
    # Inside the loop: a ray along +x crosses it an odd number of times.
    ya, yb = a[None, :, 1] - cand[:, None, 1], b[None, :, 1] - cand[:, None, 1]
    spans = (ya > 0) != (yb > 0)
    t = np.where(spans, ya / np.where(spans, ya - yb, 1.0), 0.0)
    xcut = a[None, :, 0] + (b[None, :, 0] - a[None, :, 0]) * t
    inside = ((spans & (xcut > cand[:, None, 0])).sum(1) % 2) == 1
    if not inside.any():
        return None
    cand = cand[inside]
    clear = segment_gap(cand, a, b)
    score = clear - DRIFT * np.linalg.norm(cand, axis=1)
    best = int(np.argmax(score))
    centre, rho = cand[best], float(clear[best])
    if not LIMB_MIN * h <= rho <= LIMB_MAX * h:
        return None
    mid = (a + b) / 2.0
    part = np.linalg.norm(mid - centre, axis=1) <= ROUND * rho + 0.004 * h
    return part, centre, rho


def arm_members(body, arm, mesh):
    """Which welded vertices belong to this arm (skin, sleeve, glove), and the limb's radius
    along it."""
    h = body.height
    wpos, wtris, tri_edges, edges, nwelded = mesh.pos, mesh.tris, mesh.tri_edges, mesh.edges, mesh.n
    step = PLANE * h
    planes = np.arange(0.5 * step, arm.length, step)
    arm_seed = np.zeros(nwelded, bool)
    other_seed = np.zeros(nwelded, bool)
    centres = np.full((len(planes), 3), np.nan)
    radius = np.full(len(planes), np.nan)
    cuts = []
    for s in planes:
        origin, _, e1, e2, curves = cross_section(arm, s, wpos, wtris, tri_edges, h)
        found = []
        for c in curves:
            p = limb_part(c, h)
            centre = None if p is None else origin + p[1][0] * e1 + p[1][1] * e2
            found.append((c, p, centre))
        cuts.append(found)
    # The limb's loops. Sure where a loop holds the bone, or hugs it (a bone lying on the edge
    # of its sleeve); between those, the loop that carries on from its neighbour: near where the
    # limb's middle was, about as wide.
    chosen = [[] for _ in planes]
    for i, found in enumerate(cuts):
        holding = [k for k, (c, p, _) in enumerate(found) if p is not None and c["inside"]]
        if not holding:
            hug = [(c["gap"], k) for k, (c, p, _) in enumerate(found)
                   if p is not None and c["gap"] <= max(HUG * p[2], 0.004 * h)]
            holding = [min(hug)[1]] if hug else []
        chosen[i] = holding
    sure = [bool(c) for c in chosen]

    def reference(i):
        ks = chosen[i]
        if not ks:
            return None
        k = min(ks, key=lambda k: cuts[i][k][1][2])
        return cuts[i][k][2], cuts[i][k][1][2]

    for order in (range(len(planes)), range(len(planes) - 1, -1, -1)):
        last = None
        for i in order:
            if chosen[i]:
                last = reference(i)
                continue
            if last is None:
                continue
            at, rho = last
            fits = [(float(np.linalg.norm(centre - at)), k)
                    for k, (c, p, centre) in enumerate(cuts[i])
                    if p is not None and 1.0 / CARRY <= p[2] / rho <= CARRY]
            fits = [f for f in fits if f[0] <= max(STEER * rho, 0.006 * h)]
            if fits:
                chosen[i] = [min(fits)[1]]
                last = reference(i)
            else:
                last = None
    for i, found in enumerate(cuts):
        in_hand = planes[i] >= arm.hand_from
        for k, (c, p, centre) in enumerate(found):
            if k in chosen[i]:
                part = p[0]
                arm_seed[wtris[c["tris"][part]].ravel()] = True
                other_seed[wtris[c["tris"][~part]].ravel()] = True
            elif in_hand and c["closed"] and c["reach"] <= FINGER * h:
                # A finger's loop, or a strand beside the hand: left for what it joins to.
                continue
            else:
                other_seed[wtris[c["tris"]].ravel()] = True
        ref = reference(i)
        if ref is not None:
            centres[i], radius[i] = ref
    other_seed &= ~arm_seed
    # Where no loop held the bone (the shoulder, where the sleeve joins the body), the nearest
    # plane that had one stands in.
    found = ~np.isnan(radius)
    s_w, seg_w, radial_w, above_w = arm.project(wpos)
    if found.any():
        idx = np.nonzero(found)[0]
        fill = idx[np.abs(planes[:, None] - planes[idx][None, :]).argmin(1)]
        bone_at = np.array([arm.at(s) for s in planes])
        offset = (centres - bone_at)[fill]
        radius = radius[fill]
        at = np.clip(np.searchsorted(planes, s_w), 0, len(planes) - 1)
        axes = np.array([f[0] for f in arm.frames])[seg_w]
        off = offset[at] - (offset[at] * axes).sum(1)[:, None] * axes
        r = np.linalg.norm(radial_w - off, axis=1)
        # Only where no loop held the bone: elsewhere the loops decide.
        core = ~above_w & (s_w <= arm.length) & (r <= CORE_Q * radius[at]) & ~found[at]
    else:
        radius = np.full(len(planes), np.nan)
        core = np.zeros(nwelded, bool)
    arm_seed |= core & ~other_seed
    # Spread both labels over the surface near the arm; the nearer one wins.
    capsule = ~above_w & (np.linalg.norm(radial_w, axis=1) < NEAR * h) & \
        (s_w <= arm.length + 0.05 * h)
    inner = edges[capsule[edges[:, 0]] & capsule[edges[:, 1]]]
    inner_len = np.linalg.norm(wpos[inner[:, 0]] - wpos[inner[:, 1]], axis=1)
    to_arm = surface_distance(nwelded, inner, inner_len, arm_seed & capsule, np.inf)
    to_other = surface_distance(nwelded, inner, inner_len, other_seed & capsule, np.inf)
    members = capsule & (to_arm < to_other)
    # A piece on its own that no cut gave to the rest goes with the arm whole: a sleeve or a
    # glove of its own, up to its edge above the shoulder, and a finger or a nail in the hand.
    # Only cloth that crosses the cuts beside the arm is the rest.
    island = components(nwelded, edges)
    count = island.max() + 1

    def any_in(mask):
        return np.bincount(island, weights=mask.astype(np.float64), minlength=count) > 0

    free = ~any_in(other_seed)
    in_hand = ~any_in(~capsule) & any_in(capsule & (s_w >= arm.hand_from))
    members |= (free & (any_in(members) | in_hand))[island]
    return members, radius


def hand_region(body, arm, mesh, found):
    """The hand's own surface, as welded vertices: from the arm found just before the wrist,
    spread over the mesh edges through what lies past the wrist within HAND_REACH of the bones.
    Hair or cloth beside the hand joins it only through the head or the body, outside that, so
    it is never reached."""
    h = body.height
    s, _, radial, above = arm.project(mesh.pos)
    near = ~above & (s >= arm.hand_from - HAND_WRIST * h) & \
        (np.linalg.norm(radial, axis=1) <= HAND_REACH * h)
    seed = found & near & (s <= arm.hand_from)
    inside = near[mesh.edges[:, 0]] & near[mesh.edges[:, 1]]
    dist = surface_distance(mesh.n, mesh.edges[inside], mesh.length[inside], seed, np.inf)
    return np.isfinite(dist), near


def hand_pieces(skin, body, mesh, hand, near, island, island_size, side):
    """Small islands of their own that go with the hand whole: most of the piece within
    HAND_TOUCH of the hand's surface (a ring, a nail), or most of it weighted to the hand and
    finger bones (a prop the hand holds)."""
    small = island_size <= ISLAND_MAX * mesh.n
    small[np.unique(island[hand])] = False
    touch = np.zeros(mesh.n, bool)
    look = np.nonzero(near & ~hand & small[island])[0]
    surface = mesh.pos[hand]
    if len(surface):
        for chunk in np.array_split(look, max(1, len(look) // 256)):
            gap = np.linalg.norm(mesh.pos[chunk][:, None, :] - surface[None], axis=2).min(1)
            touch[chunk] = gap <= HAND_TOUCH * body.height
    count = np.maximum(island_size, 1)
    touching = np.bincount(island, weights=touch.astype(np.float64), minlength=len(count)) / count
    bones = np.zeros(len(skin.names), bool)
    bones[sorted(skin.descendants(body.joint[side + "Hand"]))] = True
    w = skin.weights
    held = (w * bones[skin.joints]).sum(1) > HAND_HELD * w.sum(1)
    of = island[mesh.of]
    holding = np.bincount(of, weights=held.astype(np.float64), minlength=len(count)) / \
        np.maximum(np.bincount(of, minlength=len(count)), 1)
    return small & ((touching >= ISLAND_IN) | (holding >= ISLAND_IN))


def lifted(body, arm, pos):
    """Where each point goes if it follows this arm wholly when the arms are lifted level (the
    upper arm turned at the shoulder, then the lower arm at the elbow)."""
    level = body.left if arm.side == "left" else -body.left
    u, lo, hd = arm.points[:3]

    def turn(v):
        axis = np.cross(unit(v), level)
        s, c = np.linalg.norm(axis), float(unit(v) @ level)
        if s < 1e-9:
            return np.eye(3)
        k = axis / s
        cross = np.array([[0.0, -k[2], k[1]], [k[2], 0.0, -k[0]], [-k[1], k[0], 0.0]])
        return np.eye(3) + s * cross + (1.0 - c) * cross @ cross

    r1 = turn(lo - u)
    lo2, hd2 = u + r1 @ (lo - u), u + r1 @ (hd - u)
    r2 = turn(hd2 - lo2)
    _, seg, _, _ = arm.project(pos)
    upper = u + (pos - u) @ r1.T
    lower = lo2 + (upper - lo2) @ r2.T
    return np.where((seg == 0)[:, None], upper, lower)


def max_flow_cut(count, pairs, caps, source, sink):
    """Dinic's maximum flow on an undirected graph of `count` nodes; returns which nodes stay
    reachable from `source` in the residual graph (the source's side of a minimum cut)."""
    head = [[] for _ in range(count)]
    to, cap = [], []
    for (a, b), c in zip(pairs, caps):
        head[a].append(len(to))
        to.append(b)
        cap.append(c)
        head[b].append(len(to))
        to.append(a)
        cap.append(c)
    while True:
        level = [-1] * count
        level[source] = 0
        queue = [source]
        for u in queue:
            for e in head[u]:
                if cap[e] > 0 and level[to[e]] < 0:
                    level[to[e]] = level[u] + 1
                    queue.append(to[e])
        if level[sink] < 0:
            break
        nxt = [0] * count
        while True:
            # One augmenting path along the level graph, found without recursion.
            path, u = [], source
            while u != sink:
                while nxt[u] < len(head[u]):
                    e = head[u][nxt[u]]
                    if cap[e] > 0 and level[to[e]] == level[u] + 1:
                        break
                    nxt[u] += 1
                if nxt[u] == len(head[u]):
                    if not path:
                        break
                    level[u] = -1
                    u = to[path.pop() ^ 1]
                    continue
                path.append(head[u][nxt[u]])
                u = to[path[-1]]
            if u != sink:
                break
            push = min(cap[e] for e in path)
            for e in path:
                cap[e] -= push
                cap[e ^ 1] += push
    seen = np.zeros(count, bool)
    seen[source] = True
    queue = [source]
    for u in queue:
        for e in head[u]:
            if cap[e] > 0 and not seen[to[e]]:
                seen[to[e]] = True
                queue.append(to[e])
    return seen


def bridges_of(mesh, pos, lift, follows, shoulders, h):
    """Bridges: faces that join a part following an arm to a part staying with the body, so
    lifting the arms must stretch them (an auto rig's mesh fused where a sleeve rested on the
    hip). Each vertex is moved by the share of its weight on each arm; faces stretched past
    STRETCHED form patches, and a patch is a bridge when it lies further than BRIDGE_FROM from
    the shoulder and stretches longer than BRIDGE_LONG (the armpit stretching is the lift
    itself). Returns them largest first, each with its side, face count, middle (bind space),
    length (the longest stretched edge once lifted, in metres) and its triangles in the skin."""
    moved = pos.copy()
    for side in SIDES:
        moved += follows[side][:, None] * (lift[side] - pos)
    a, b = np.zeros((mesh.n, 3)), np.zeros((mesh.n, 3))
    a[mesh.of], b[mesh.of] = pos, moved
    before = np.linalg.norm(a[mesh.edges[:, 0]] - a[mesh.edges[:, 1]], axis=1)
    after = np.linalg.norm(b[mesh.edges[:, 0]] - b[mesh.edges[:, 1]], axis=1)
    bad = (after > STRETCHED * before) & (after > 0.02 * h)
    faces = np.nonzero(bad[mesh.tri_edges].any(1))[0]
    if len(faces) == 0:
        return []
    moves = {}
    for side in SIDES:
        moves[side] = np.zeros(mesh.n)
        np.maximum.at(moves[side], mesh.of, follows[side])
    tris = mesh.tris[faces]
    label = components(mesh.n, np.concatenate([tris[:, [0, 1]], tris[:, [1, 2]]]))[tris[:, 0]]
    out = []
    for k in np.unique(label):
        mine = faces[label == k]
        corners = mesh.tris[mine]
        side = max(SIDES, key=lambda s: float(moves[s][corners].sum()))
        at = mesh.pos[corners].mean((0, 1))
        length = float(after[mesh.tri_edges[mine]].max())
        if np.linalg.norm(at - shoulders[side]) <= BRIDGE_FROM * h or length <= BRIDGE_LONG * h:
            continue
        out.append({
            "side": side,
            "faces": int(len(mine)),
            "at": [round(float(x), 3) for x in at],
            "length": round(length, 3),
            "tris": mesh.source[mine],
        })
    return sorted(out, key=lambda p: -p["faces"])


def split_band(mesh, share, reach, weighted, members, h):
    """Where lifting the arm moves the band far, a share kept on the body stretches it into a
    sheet (cloth an auto rig fused between a sleeve and the hip). There each band vertex goes
    wholly to the arm or to the body, split along the seam that costs least to stretch: a
    minimum cut over the mesh edges, each costing its length times how far the lift moves it.
    Returns the vertices that go to the arm. The faces left spanning the split must stretch:
    they bridge two parts that move apart."""
    of = mesh.of
    far = np.zeros(mesh.n)
    np.maximum.at(far, of, reach)
    part = np.zeros(mesh.n)
    np.maximum.at(part, of, np.where(weighted & ~members, share, 0.0))
    arm_end = np.zeros(mesh.n, bool)
    arm_end[of[members]] = True
    free = (part > 0.0) & (part < 1.0) & (far > SPLIT_LIFT * h) & ~arm_end
    if not free.any():
        return np.zeros(len(share), bool)
    # The ends: the arm's surface and the band near enough the shoulder to keep its share
    # (on the arm's side when it keeps most), and everything else on the body's side.
    arm_end |= ~free & (part >= 0.5)
    node = np.full(mesh.n, -1)
    node[free] = np.arange(int(free.sum()))
    source, sink = int(free.sum()), int(free.sum()) + 1
    a, b = mesh.edges[:, 0], mesh.edges[:, 1]
    touch = free[a] | free[b]
    cost = mesh.length * (far[a] + far[b]) / 2.0 / (h * h)
    caps = np.maximum(np.rint(cost * 1e6), 1).astype(np.int64)
    ends = lambda v: np.where(free[v], node[v], np.where(arm_end[v], source, sink))
    na, nb = ends(a[touch]), ends(b[touch])
    keep = na != nb
    side = max_flow_cut(source + 2, np.stack([na[keep], nb[keep]], 1).tolist(),
                        caps[touch][keep].tolist(), source, sink)
    to_arm = free & side[np.maximum(node, 0)]
    return to_arm[of]


def follow_arm(weights, on_arm, members, reach, h):
    """The arm's own surface (sleeve, glove, hand) follows the arm wholly where lifting it
    moves far: weight an auto rig gave it on the body or the legs (a sleeve resting on the hip)
    would hold it back and stretch it. Near the shoulder the blend with the body stays. Returns
    the weights and which vertices changed."""
    out = weights.copy()
    followed = np.zeros(len(weights), bool)
    for side in SIDES:
        m = on_arm[side]
        arm = np.where(m, out, 0.0).sum(1)
        other = out.sum(1) - arm
        share = smoothstep((reach[side] - FOLLOW_LIFT * h) / (FOLLOW_LIFT * h))
        rows = members[side] & (arm > 0) & (share * other > CHANGED)
        if not rows.any():
            continue
        gain = (share * other)[rows] / arm[rows]
        out[rows] = np.where(m[rows], out[rows] * (1.0 + gain[:, None]),
                             out[rows] * (1.0 - share[rows])[:, None])
        followed |= rows
    return out, followed


def trunk_share(body, pos):
    """For each vertex, how the trunk bones share weight moved off an arm: inverse distance to
    the spine, chest and upperChest, and to each shoulder above the armpit."""
    h = body.height
    chain = [r for r in ("spine", "chest", "upperChest") if body.has(r)]
    end = next((r for r in ("neck", "head") if body.has(r)), None)
    segs = []
    for i, role in enumerate(chain):
        nxt = chain[i + 1] if i + 1 < len(chain) else end
        b = body.at(nxt) if nxt else body.at(role) + body.up * 0.1 * h
        segs.append((body.joint[role], body.at(role), b, None))
    for side in SIDES:
        if body.has(side + "Shoulder"):
            top = float(body.at(side + "UpperArm") @ body.up)
            segs.append((body.joint[side + "Shoulder"], body.at(side + "Shoulder"),
                         body.at(side + "UpperArm"), top))
    heights = pos @ body.up
    inv = []
    for _, a, b, top in segs:
        d = np.maximum(segment_distance(pos, a, b), 0.01 * h)
        w = 1.0 / d ** IDW_POWER
        if top is not None:
            w *= smoothstep((heights - (top - ARMPIT * h)) / (ARMPIT * h))
        inv.append(w)
    inv = np.stack(inv, 1)
    # The two nearest trunk bones only, so the trunk never crowds a kept arm weight out of the
    # four influences (which would leave a vertex behind its neighbours when the arm lifts).
    if inv.shape[1] > TRUNK_BONES:
        inv[inv < np.sort(inv, 1)[:, -TRUNK_BONES][:, None]] = 0.0
    inv /= np.maximum(inv.sum(1, keepdims=True), 1e-30)
    return [j for j, _, _, _ in segs], inv


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


def reweight_skin(skin, bones):
    body = Body(skin, bones)
    h = body.height
    pos, tris = skin.pos, skin.tris
    n = len(pos)
    mesh = Welded(pos, tris, 1e-5 * h)
    island = components(mesh.n, mesh.edges)
    island_size = np.bincount(island)
    joints, weights = skin.joints, skin.weights
    arms = {side: Arm(body, side) for side in SIDES}
    on_arm = {}
    for side, arm in arms.items():
        chain = np.zeros(len(skin.names), bool)
        chain[arm.chain] = True
        on_arm[side] = chain[joints] & (weights > 0)
    arm_weight = {side: (weights * m).sum(1) for side, m in on_arm.items()}
    members = {}
    info = {}
    for side, arm in arms.items():
        found, radius = arm_members(body, arm, mesh)
        # Small islands mostly on the arm (a cuff, a bracelet) go with it whole.
        share = np.bincount(island, weights=found.astype(np.float64), minlength=len(island_size))
        share /= np.maximum(island_size, 1)
        cuffs = (share >= ISLAND_IN) & (island_size <= ISLAND_MAX * mesh.n)
        hand, near = hand_region(body, arm, mesh, found)
        pieces = hand_pieces(skin, body, mesh, hand, near, island, island_size, side)
        members[side] = (found | hand | cuffs[island] | pieces[island])[mesh.of]
        info[side] = {"radius": radius}
    # The share of its arm weight each vertex keeps: all on the arm, fading over the band. The
    # band only blends where lifting the arm moves the surface little (near the shoulder);
    # further down a share would stretch into a sheet between the sleeve and the body, so
    # there each vertex goes wholly to the arm or to the body, split along the narrowest seam.
    keep = {}
    reach = {}
    bridges = {}
    lift = {}
    for side, arm in arms.items():
        lift[side] = lifted(body, arm, pos)
        reach[side] = np.linalg.norm(lift[side] - pos, axis=1)
        sources = np.zeros(mesh.n, bool)
        sources[mesh.of[members[side]]] = True
        dist = surface_distance(mesh.n, mesh.edges, mesh.length, sources, BAND * h)[mesh.of]
        share = np.where(np.isfinite(dist), smoothstep(1.0 - dist / (BAND * h)), 0.0)
        share[members[side]] = 1.0
        arm_side = split_band(mesh, share, reach[side], arm_weight[side] > 0, members[side], h)
        members[side] |= arm_side
        share[arm_side] = 1.0
        share[(reach[side] > SPLIT_LIFT * h) & ~members[side]] = 0.0
        keep[side] = share
    weights, followed = follow_arm(weights, on_arm, members, reach, h)
    trunk_joints, trunk = trunk_share(body, pos)
    new_w = weights.copy()
    moved = np.zeros(n)
    from_bone = {}
    for side in SIDES:
        m = on_arm[side]
        lose = (1.0 - keep[side])[:, None] * np.where(m, weights, 0.0)
        new_w -= lose
        moved += lose.sum(1)
        taken = lose.sum(1) > CHANGED
        if taken.any():
            jj = np.where(m, joints, -1)[taken]
            ll = lose[taken]
            top = jj[np.arange(len(jj)), ll.argmax(1)]
            for jnt, count in zip(*np.unique(top, return_counts=True)):
                name = skin.names[int(jnt)]
                from_bone[name] = from_bone.get(name, 0) + int(count)
    extra_j = np.tile(np.array(trunk_joints, np.int64), (n, 1))
    extra_w = trunk * moved[:, None]
    all_j = np.concatenate([joints, extra_j], 1)
    all_w = np.concatenate([new_w, extra_w], 1)
    # Vertices with no weight at all take the trunk whole.
    empty = weights.sum(1) <= 0
    all_w[empty, joints.shape[1]:] = trunk[empty]
    out_j, out_w = top_four(all_j, all_w)
    follows = {}
    for side in SIDES:
        chain = np.zeros(len(skin.names), bool)
        chain[arms[side].chain] = True
        follows[side] = (out_w * chain[out_j]).sum(1)
    shoulders = {side: body.at(side + "UpperArm") for side in SIDES}
    bridges = bridges_of(mesh, pos, lift, follows, shoulders, h)
    changed = (moved > CHANGED) | empty | ((weights > 0).sum(1) > MAX_INFLUENCES)
    changed |= (np.abs(weights.sum(1) - 1.0) > 1e-3) | followed
    to_bone = {}
    if (moved > CHANGED).any():
        gains = np.zeros(len(trunk_joints))
        for k in range(len(trunk_joints)):
            gains[k] = float((extra_w[:, k] > CHANGED).sum())
        for jnt, count in zip(trunk_joints, gains):
            if count:
                to_bone[skin.names[jnt]] = int(count)
    stats = {
        "vertices": int(n),
        "armWeighted": int(sum((arm_weight[s] > 0).sum() for s in SIDES)),
        "moved": int((moved > CHANGED).sum()),
        "weightMoved": round(float(moved.sum()), 3),
        "keptAsSleeve": int(sum((members[s] & (arm_weight[s] > 0)).sum() for s in SIDES)),
        "band": int(sum(((keep[s] > 0) & (keep[s] < 1) & (arm_weight[s] > 0)).sum()
                        for s in SIDES)),
        "followArm": int(followed.sum()),
        "bridgeFaces": int(sum(p["faces"] for p in bridges)),
        "bridges": [{k: v for k, v in p.items() if k != "tris"} for p in bridges[:BRIDGES_LISTED]],
        "fromBones": dict(sorted(from_bone.items(), key=lambda kv: -kv[1])),
        "toBones": dict(sorted(to_bone.items(), key=lambda kv: -kv[1])),
        "sides": {
            side: {
                "moved": int(((1.0 - keep[side]) * arm_weight[side] > CHANGED).sum()),
                "keptAsSleeve": int((members[side] & (arm_weight[side] > 0)).sum()),
                "limbRadius": [round(float(x) / h, 4) for x in info[side]["radius"] if x == x],
                "bridgeFaces": int(sum(p["faces"] for p in bridges if p["side"] == side)),
            }
            for side in SIDES
        },
    }
    debug = {"members": members, "keep": keep, "info": info, "armWeight": arm_weight,
             "bridgeTris": np.concatenate([p["tris"] for p in bridges] or [np.zeros(0, np.int64)]),
             "moved": moved}
    return out_j, out_w, changed, stats, debug


def weight_checks(joints, weights):
    total = weights.sum(1)
    count = (weights > 0).sum(1)
    return {
        "vertices": int(len(weights)),
        "maxInfluences": int(count.max()) if len(count) else 0,
        "unweighted": int((total <= 0).sum()),
        "offSum": int((np.abs(total - 1.0) > 1e-3).sum()),
    }


def reweight(data, bones, hide=True):
    """Reweights the model in `data` (GLB bytes), hiding the bridge faces when `hide`; returns
    the new bytes, the report and what each skin decided (for a look)."""
    glb = Glb(bytes(data))
    skins = skins_of(glb)
    if not skins:
        raise Refused("no mesh is skinned")
    report = None
    decided = []
    for skin in skins:
        try:
            joints, weights, rows, stats, debug = reweight_skin(skin, bones)
        except Refused:
            if len(skins) == 1:
                raise
            continue
        skin.store(glb, joints, weights, rows)
        stats["weights"] = weight_checks(joints, weights)
        stats["hiddenFaces"] = hide_faces(glb, skin, debug["bridgeTris"]) if hide else 0
        report = stats if report is None else merge_reports(report, stats)
        decided.append((skin, debug))
    if report is None:
        raise Refused("no skin carries the mapped arm bones")
    out = glb.bytes() if report["hiddenFaces"] else bytes(glb.data)
    return out, report, decided


def merge_reports(a, b):
    out = dict(a)
    for key in ("vertices", "armWeighted", "moved", "keptAsSleeve", "band", "followArm",
                "bridgeFaces", "hiddenFaces"):
        out[key] = a[key] + b[key]
    out["bridges"] = sorted(a["bridges"] + b["bridges"], key=lambda p: -p["faces"])[:BRIDGES_LISTED]
    out["sides"] = {
        side: {
            **a["sides"][side],
            **{k: a["sides"][side][k] + b["sides"][side][k]
               for k in ("moved", "keptAsSleeve", "bridgeFaces")},
        }
        for side in SIDES
    }
    out["weightMoved"] = round(a["weightMoved"] + b["weightMoved"], 3)
    for key in ("fromBones", "toBones"):
        merged = dict(a[key])
        for k, v in b[key].items():
            merged[k] = merged.get(k, 0) + v
        out[key] = dict(sorted(merged.items(), key=lambda kv: -kv[1]))
    out["weights"] = {
        "vertices": a["weights"]["vertices"] + b["weights"]["vertices"],
        "maxInfluences": max(a["weights"]["maxInfluences"], b["weights"]["maxInfluences"]),
        "unweighted": a["weights"]["unweighted"] + b["weights"]["unweighted"],
        "offSum": a["weights"]["offSum"] + b["weights"]["offSum"],
    }
    return out


# Sheets, in Blender.

SHEET_POSES = ["rest", "level", "raise"]
SHEET_VIEWS = ["front", "side", "back", "shoulders"]
CELL = 400
RAISE_DEG = 45.0


def sheets(job, before_path, after_path):
    import bpy
    from mathutils import Matrix, Quaternion, Vector

    def update():
        bpy.context.view_layer.update()

    class Rig:
        def __init__(self, roles):
            for obj in list(bpy.data.objects):
                if any(c.name == "glTF_not_exported" for c in obj.users_collection):
                    bpy.data.objects.remove(obj)
            arms = [o for o in bpy.data.objects if o.type == "ARMATURE"]
            if len(arms) != 1:
                fail(f"expected one armature, found {len(arms)}")
            self.arm = arms[0]
            self.meshes = [o for o in bpy.data.objects if o.type == "MESH" and any(
                m.type == "ARMATURE" and m.object == self.arm for m in o.modifiers)]
            self.roles = {r: b for r, b in roles.items() if b in self.arm.data.bones}

        def head(self, role):
            pb = self.arm.pose.bones.get(self.roles.get(role, ""))
            return self.arm.matrix_world @ pb.head if pb else None

        def reset(self):
            for pb in self.arm.pose.bones:
                pb.matrix_basis = Matrix.Identity(4)
            if self.arm.animation_data:
                self.arm.animation_data.action = None
            update()

        def aim(self, role, tip_role, direction):
            pb = self.arm.pose.bones.get(self.roles.get(role, ""))
            a, b = self.head(role), self.head(tip_role)
            if pb is None or a is None or b is None:
                return
            rot = (b - a).normalized().rotation_difference(Vector(direction).normalized())
            world = self.arm.matrix_world @ pb.matrix
            world = Matrix.Translation(a) @ rot.to_matrix().to_4x4() @ Matrix.Translation(-a) @ world
            pb.matrix = self.arm.matrix_world.inverted() @ world
            update()

        def points(self):
            depsgraph = bpy.context.evaluated_depsgraph_get()
            out = []
            for obj in self.meshes:
                ev = obj.evaluated_get(depsgraph)
                me = ev.to_mesh()
                co = np.empty(len(me.vertices) * 3)
                me.vertices.foreach_get("co", co)
                mw = np.array(obj.matrix_world)
                out.append(co.reshape(-1, 3) @ mw[:3, :3].T + mw[:3, 3])
                ev.to_mesh_clear()
            return np.concatenate(out)

    def load(path):
        bpy.ops.wm.read_factory_settings(use_empty=True)
        bpy.ops.import_scene.gltf(filepath=path)
        rig = Rig(job["bones"])
        rig.reset()
        # Face the camera: the character's left (left minus right limbs) onto +X.
        total = Vector((0.0, 0.0, 0.0))
        for part in ("UpperLeg", "UpperArm"):
            a, b = rig.head("left" + part), rig.head("right" + part)
            if a is not None and b is not None:
                d = Vector((a.x - b.x, a.y - b.y, 0.0))
                if d.length > 1e-6:
                    total += d.normalized()
        turn = -math.atan2(total.y, total.x) if total.length > 1e-6 else 0.0
        rz = Matrix.Rotation(turn, 4, "Z")
        for obj in [rig.arm] + [m for m in rig.meshes if m.parent is None]:
            obj.matrix_world = rz @ obj.matrix_world
        update()
        hips = rig.head("hips")
        pts = rig.points()
        move = Matrix.Translation(Vector((-hips.x, -hips.y, -float(pts[:, 2].min()))))
        for obj in [rig.arm] + [m for m in rig.meshes if m.parent is None]:
            obj.matrix_world = move @ obj.matrix_world
        update()
        return rig

    def pose(rig, name):
        rig.reset()
        if name == "rest":
            return
        deg = 0.0 if name == "level" else RAISE_DEG
        r = math.radians(deg)
        for side, x in (("left", 1.0), ("right", -1.0)):
            d = (x * math.cos(r), 0.0, math.sin(r))
            rig.aim(side + "UpperArm", side + "LowerArm", d)
            rig.aim(side + "LowerArm", side + "Hand", d)

    def frame_for(rig):
        pts = rig.points()
        height = float(pts[:, 2].max() - pts[:, 2].min())
        reach = 0.0
        for side in SIDES:
            u, lo, hd = (rig.head(side + b) for b in ("UpperArm", "LowerArm", "Hand"))
            reach = max(reach, abs(u.x) + (lo - u).length + (hd - lo).length + 0.12 * height)
        body = max(1.08 * height, 2.1 * reach)
        z = rig.head("leftUpperArm").z
        return {"body": (body, height / 2), "shoulders": (0.5 * height, z - 0.05 * height)}

    def setup():
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
        return cam, label

    def place(cam, label, view, frame, text):
        scale, z = frame["shoulders"] if view == "shoulders" else frame["body"]
        yaw = math.radians({"front": 0.0, "shoulders": 0.0, "side": 90.0, "back": 180.0}[view])
        d = Vector((math.sin(yaw), -math.cos(yaw), 0.0))
        centre = Vector((0.0, 0.0, z))
        cam.data.ortho_scale = scale
        cam.location = centre + d * 10.0
        cam.rotation_euler = (-d).to_track_quat("-Z", "Y").to_euler()
        right = Vector((math.cos(yaw), math.sin(yaw), 0.0))
        label.data.body = text
        label.data.size = 0.03 * scale
        label.location = centre + d * 5.0 - right * 0.46 * scale + Vector((0.0, 0.0, 0.44 * scale))
        label.rotation_euler = (math.radians(90.0), 0.0, yaw)

    def render(path, out, title, frame):
        rig = load(path)
        frame = frame or frame_for(rig)
        cam, label = setup()
        cells = out + "_cells"
        os.makedirs(cells, exist_ok=True)
        sheet = np.ones((CELL * len(SHEET_POSES), CELL * len(SHEET_VIEWS), 4), np.float32)
        for ri, name in enumerate(SHEET_POSES):
            pose(rig, name)
            for ci, view in enumerate(SHEET_VIEWS):
                place(cam, label, view, frame, f"{title}  {name}  {view}")
                cell = os.path.join(cells, f"{name}_{view}.png")
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
        img = bpy.data.images.new("sheet", sheet.shape[1], sheet.shape[0], alpha=True)
        img.pixels.foreach_set(sheet[::-1].ravel())
        img.filepath_raw = out
        img.file_format = "PNG"
        img.save()
        bpy.data.images.remove(img)
        log(f"sheet {out}")
        return frame

    frame = None
    if "before" in job["sheets"]:
        frame = render(before_path, job["sheets"]["before"], "before", frame)
    if "after" in job["sheets"]:
        render(after_path, job["sheets"]["after"], "after", frame)


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

    for key in ("input", "output", "report"):
        job[key] = resolve(job[key])
    job["sheets"] = {k: resolve(v) for k, v in job.get("sheets", {}).items()}
    return job


def main():
    job = load_job(parse_args().job)
    with open(job["input"], "rb") as fh:
        data = fh.read()
    try:
        out, report, _ = reweight(data, job["bones"], job.get("bridges", "hide") == "hide")
    except Refused as e:
        fail(str(e))
    with open(job["output"], "wb") as fh:
        fh.write(out)
    log(f"{report['moved']} vertices moved weight off the arms, "
        f"{report['keptAsSleeve']} kept as sleeve")
    if report["hiddenFaces"]:
        log(f"{report['hiddenFaces']} bridge faces hidden")
    if job["sheets"]:
        try:
            import bpy  # noqa: F401
        except ImportError:
            fail("the sheets need Blender; run this with blender -b")
        sheets(job, job["input"], job["output"])
    with open(job["report"], "w", encoding="utf-8") as fh:
        json.dump(report, fh, indent=2)
    log(f"wrote {job['output']}")


if __name__ == "__main__":
    try:
        main()
    except SystemExit:
        raise
    except Exception:
        traceback.print_exc()
        fail("reweighting stopped: " + traceback.format_exc().strip().splitlines()[-1])
