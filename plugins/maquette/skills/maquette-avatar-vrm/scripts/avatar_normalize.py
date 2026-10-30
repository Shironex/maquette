"""Normalise a rigged character's rest pose for VRM: a T-pose facing +Z.

Maquette runs this headless, never with a window, on a copy of the model:

    blender -b --factory-startup --python-exit-code 1 --python avatar_normalize.py -- \
        --job job.json

job.json names the model, where the outputs go and the VRM bone roles, each
a VRM humanoid bone name to the joint (bone) name it maps to, as the VRM
readiness found them. Relative paths resolve against the job file's folder:

    {"input": "/abs/model.glb", "output": "normalized.glb", "report": "report.json",
     "sheets": {"before": "sheet_before.png", "after": "sheet_after.png"},
     "bones": {"hips": "mixamorig:Hips", "leftUpperArm": "mixamorig:LeftArm", ...},
     "elbows": "straighten", "fingers": "straighten"}

What it does, in this order:

1. Turn the armature and the meshes about the up axis so the face points
   to glTF +Z and the character's left lands on +X. The left comes from the
   left minus right upper legs and upper arms, as the readiness reads it.
2. Move them so the hips sit on the Y axis and the soles on Y = 0, and
   bake the turn and the move into the armature and the meshes.
3. Lift each arm at the shoulder (the upper arm's head) until upper arm to
   hand is level along X. The clavicles stay where they are and the hand
   and fingers ride along unchanged. With "elbows": "straighten" (the
   default) the lower arm is levelled too, so upper arm, lower arm and hand
   all lie on X; "keep" leaves a bent elbow bent.
4. With "fingers": "straighten" (the default), lay each finger straight
   along the hand in the palm plane, as VRM's T-pose wants: the four
   fingers keep their natural spread (half the fan the knuckles make from
   the wrist), the thumb points 45 degrees between the hand and the front.
   Generated hands often have fused fingers that tear when pulled apart, so
   each hand goes only as far as its mesh holds: the whole way, else three
   quarters, half or a quarter of each joint's turn, the first whose finger
   edges stretch at most STRETCH_MAX (95th percentile), leaving a soft curl.
   A hand whose finger bones sit bunched at the wrist (a broken auto rig)
   is repaired first: the five fingers are found on the hand's surface (the
   tips stand out farthest from the wrist, the thumb is the one that parts
   from the rest nearest the wrist), each finger's joints are placed along
   its finger, and the hand is reweighted with a smooth falloff along every
   finger and across the seams between them (at most 4 influences, summing
   to 1). A hand where five fingers cannot be told apart stays as it is.
   "keep" leaves every finger as it is.
5. Make that pose the rest: the armature modifier is applied on each
   skinned mesh, the pose becomes the armature's rest, and the modifier is
   added back. The vertex groups stay as they were; at most 4 influences
   per vertex and weights summing to 1 are checked and kept.
6. Export a plain GLB without animations.

The report holds the measures before and after (facing, each arm's drop,
the elbow bends, each finger's curl, the soles, the hips, the height) in glTF metres and
degrees, the turn and the move, the weight checks, and per repaired hand
the vertices reweighted and how far the middle finger reached from the wrist
before and after. The two pose sheets
show the model before (only turned to face the camera) and after, in rows
rest, arms down, arms raised and a forward step, and columns front, side,
back and a close look at the shoulders, so the armpits and whatever hangs
beside the arms (a cape, hair) can be judged. A last row looks at the
hands at rest up close: each hand from its thumb side, where a curl shows,
and from the back of the hand, where fused or torn fingers show. Poses are world space turns
about each bone's head, so they read the same on any rig.
"""

import argparse
import heapq
import json
import math
import os
import sys
import traceback

import bpy
import numpy as np
from mathutils import Matrix, Quaternion, Vector

TAG = "[avatar]"
MAX_INFLUENCES = 4
SHEET_POSES = ["rest", "down", "raise", "step"]
SHEET_VIEWS = ["front", "side", "back", "shoulders"]
# The hands row: each hand from its thumb side and from its back.
HAND_VIEWS = [("left", "front"), ("left", "top"), ("right", "front"), ("right", "top")]
CELL = 400
# Degrees the arms point below and above level in the down and raise poses.
DOWN_DEG = 65.0
RAISE_DEG = 45.0
FINGERS = ["Thumb", "Index", "Middle", "Ring", "Little"]
# How far a hand's fingers are straightened, tried in this order.
SHARES = (1.0, 0.75, 0.5, 0.25)
# The finger edges may stretch or shrink this much (95th percentile, as a
# ratio) when straightened; beyond it fused fingers tear.
STRETCH_MAX = 1.35
# Finger edges are those whose ends both carry this much finger weight.
FINGER_WEIGHT = 0.1
# A hand whose middle knuckle sits nearer the wrist than this share of the
# height has its finger bones bunched up, as a broken auto rig leaves them.
BUNCHED = 0.015
# The hand crops frame at least this share of the height.
HAND_MIN = 0.07
# The share of the knuckles' fan from the wrist the straightened fingers keep.
SPREAD_KEEP = 0.5
# Repairing bunched fingers, in shares of the height: the hand's surface lies
# past the wrist within REPAIR_REACH of it, reached over the mesh from what
# lies within REPAIR_WRIST of the wrist's plane and REPAIR_RING of the forearm's
# line; a finger tip lies at least REPAIR_TIP from the wrist over the surface
# and stands out at least REPAIR_KEEP from where it joins a taller one.
REPAIR_REACH = 0.14
REPAIR_WRIST = 0.01
REPAIR_RING = 0.03
REPAIR_TIP = 0.025
REPAIR_KEEP = 0.004
# A joint is the middle of the finger's surface around its distance from the
# wrist, weighted over this share of the tip's distance; the weights blend over this share of the shorter bone
# beside each joint, and fade in from the old ones over REPAIR_BLEND from the
# wrist.
REPAIR_SLICE = 0.08
REPAIR_FADE = 0.3
REPAIR_BLEND = 0.012
# Skin within this (surface distance, share of the height) of the seam
# between two fingers blends into the other one, half at the seam.
REPAIR_MIX = 0.012
# A piece of its own up to this share of the vertices, most of it within
# REPAIR_TOUCH of a finger, follows the finger (a ring).
REPAIR_PIECE = 0.02
REPAIR_TOUCH = 0.006
# Each finger's joints, its first joint's head to its tip, as shares of the
# tip's surface distance from the wrist.
FINGER_LEVELS = (0.42, 0.66, 0.82, 1.0)
THUMB_LEVELS = (0.30, 0.56, 0.79, 1.0)


def log(text):
    print(f"{TAG} {text}", flush=True)


def fail(text):
    print(f"{TAG} Error: {text}", flush=True)
    sys.exit(1)


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument("--job", required=True)
    return parser.parse_args(argv)


def load_job(path):
    with open(path, encoding="utf-8") as fh:
        job = json.load(fh)
    base = os.path.dirname(os.path.abspath(path))

    def resolve(p):
        return p if os.path.isabs(p) else os.path.join(base, p)

    job["input"] = resolve(job["input"])
    job["output"] = resolve(job["output"])
    job["report"] = resolve(job["report"])
    job["sheets"] = {k: resolve(v) for k, v in job.get("sheets", {}).items()}
    job.setdefault("elbows", "straighten")
    if job["elbows"] not in ("keep", "straighten"):
        fail(f"elbows must be keep or straighten, not {job['elbows']}")
    job.setdefault("fingers", "straighten")
    if job["fingers"] not in ("keep", "straighten"):
        fail(f"fingers must be keep or straighten, not {job['fingers']}")
    return job


# Coordinates: Blender is Z up with the front at -Y; glTF is Y up with the
# front at +Z. A point (x, y, z) in Blender is (x, z, -y) in glTF.


def to_gltf(v):
    return [v[0], v[2], -v[1]]


def update():
    bpy.context.view_layer.update()


class Rig:
    """The imported armature, its skinned meshes and the bone roles."""

    def __init__(self, roles):
        for obj in list(bpy.data.objects):
            # The importer's bone display shape is never exported.
            if any(c.name == "glTF_not_exported" for c in obj.users_collection):
                bpy.data.objects.remove(obj)
        arms = [o for o in bpy.data.objects if o.type == "ARMATURE"]
        if len(arms) != 1:
            fail(f"expected one armature, found {len(arms)}")
        self.arm = arms[0]
        self.meshes = [
            o
            for o in bpy.data.objects
            if o.type == "MESH"
            and any(m.type == "ARMATURE" and m.object == self.arm for m in o.modifiers)
        ]
        if not self.meshes:
            fail("no mesh is skinned to the armature")
        for o in bpy.data.objects:
            if o.type == "MESH" and o not in self.meshes and o.parent_type == "BONE":
                fail(f"{o.name} hangs on a bone without skinning; this script moves only skinned meshes")
        for o in self.meshes:
            if o.data.shape_keys is not None:
                fail(f"{o.name} has shape keys; applying the pose as the rest needs a mesh without them")
        self.roles = {}
        for role, bone in roles.items():
            if bone in self.arm.data.bones:
                self.roles[role] = bone
        for role in ("hips", "leftUpperArm", "rightUpperArm", "leftLowerArm", "rightLowerArm",
                     "leftHand", "rightHand", "leftUpperLeg", "rightUpperLeg"):
            if role not in self.roles:
                fail(f"the bone map has no {role}")

    def pose_bone(self, role):
        name = self.roles.get(role)
        return self.arm.pose.bones.get(name) if name else None

    def head(self, role):
        """A bone's head in world space, as posed now."""
        pb = self.pose_bone(role)
        return self.arm.matrix_world @ pb.head if pb else None

    def tip(self, role):
        """Where a bone ends in world space, as posed now: its first child's
        head (a finger's end bone), else its tail."""
        pb = self.pose_bone(role)
        if pb is None:
            return None
        end = pb.children[0].head if pb.children else pb.tail
        return self.arm.matrix_world @ end

    def mesh_points(self):
        """Every skinned mesh vertex in world space, as deformed now."""
        depsgraph = bpy.context.evaluated_depsgraph_get()
        out = []
        for obj in self.meshes:
            ev = obj.evaluated_get(depsgraph)
            me = ev.to_mesh()
            co = np.empty(len(me.vertices) * 3, np.float64)
            me.vertices.foreach_get("co", co)
            co = co.reshape(-1, 3)
            mw = np.array(obj.matrix_world)
            out.append(co @ mw[:3, :3].T + mw[:3, 3])
            ev.to_mesh_clear()
        return np.concatenate(out)

    def reset_pose(self):
        for pb in self.arm.pose.bones:
            pb.matrix_basis = Matrix.Identity(4)
        if self.arm.animation_data:
            self.arm.animation_data.action = None
        update()

    def world_rotate(self, role, rot):
        """Turns a bone by `rot` (a world rotation) about its head."""
        pb = self.pose_bone(role)
        if pb is None:
            return
        update()
        h = self.arm.matrix_world @ pb.head
        inv = self.arm.matrix_world.inverted()
        world = self.arm.matrix_world @ pb.matrix
        world = Matrix.Translation(h) @ rot.to_matrix().to_4x4() @ Matrix.Translation(-h) @ world
        pb.matrix = inv @ world
        update()

    def aim(self, role, tip_role, direction):
        """Turns `role` about its head so its head to `tip_role` points along `direction`."""
        a, b = self.head(role), self.head(tip_role)
        if a is None or b is None:
            return 0.0
        d = (b - a).normalized()
        rot = d.rotation_difference(Vector(direction).normalized())
        self.world_rotate(role, rot)
        return math.degrees(rot.angle)

    def aim_chain(self, role, direction, share):
        """Turns `role` about its head, its end being its child's head or its
        tail, `share` of the way to pointing along `direction`."""
        a = self.head(role)
        b = self.tip(role)
        if a is None or b is None or (b - a).length < 1e-9:
            return 0.0
        rot = (b - a).normalized().rotation_difference(Vector(direction).normalized())
        axis, angle = rot.to_axis_angle()
        rot = Quaternion(axis, angle * share)
        self.world_rotate(role, rot)
        return math.degrees(rot.angle)


def left_axis(rig):
    """The character's left, flattened, in Blender space."""
    total = Vector((0.0, 0.0, 0.0))
    for side in ("UpperLeg", "UpperArm"):
        l, r = rig.head("left" + side), rig.head("right" + side)
        d = Vector((l.x - r.x, l.y - r.y, 0.0))
        if d.length > 1e-6:
            total += d.normalized()
    if total.length < 1e-6:
        fail("the left and right limbs sit on top of each other; the facing cannot be read")
    return total.normalized()


def angle_between(a, b):
    if a.length < 1e-9 or b.length < 1e-9:
        return None
    return math.degrees(a.angle(b))


def measure(rig):
    """The rest pose measures the VRM readiness reads, in glTF terms."""
    update()
    left = left_axis(rig)
    lg = to_gltf(left)
    front = (-lg[2], 0.0, lg[0])
    facing = math.degrees(math.atan2(front[0], front[2]))
    arms = {}
    elbows = {}
    for side in ("left", "right"):
        u, lo, h = (rig.head(side + b) for b in ("UpperArm", "LowerArm", "Hand"))
        d = to_gltf(h - u)
        flat = math.hypot(d[0], d[2])
        arms[side] = math.degrees(math.atan2(-d[1], flat))
        elbows[side] = angle_between(lo - u, h - lo)
    pts = rig.mesh_points()
    hips = to_gltf(rig.head("hips"))
    curls = finger_curls(rig, float(pts[:, 2].max() - pts[:, 2].min()))
    return {
        "facingDeg": facing,
        "armLeftDeg": arms["left"],
        "armRightDeg": arms["right"],
        "elbowLeftDeg": elbows["left"],
        "elbowRightDeg": elbows["right"],
        "fingers": curls,
        "fingerCurlDeg": max(curls.values()) if curls else None,
        "groundY": float(pts[:, 2].min()),
        "height": float(pts[:, 2].max() - pts[:, 2].min()),
        "hips": hips,
        "hipsOffset": math.hypot(hips[0], hips[2]),
        "vertices": int(len(pts)),
    }


def select_only(objs, active):
    if bpy.context.object and bpy.context.object.mode != "OBJECT":
        bpy.ops.object.mode_set(mode="OBJECT")
    for o in bpy.context.view_layer.objects:
        o.select_set(o in objs)
    bpy.context.view_layer.objects.active = active


def orient(rig):
    """Turns the face to glTF +Z, centres the hips and grounds the soles, baked in."""
    left = left_axis(rig)
    turn = -math.atan2(left.y, left.x)
    rz = Matrix.Rotation(turn, 4, "Z")
    # Free the meshes from the armature so each object's transform bakes on its own.
    for obj in rig.meshes:
        mw = obj.matrix_world.copy()
        obj.parent = None
        obj.matrix_world = mw
    for obj in [rig.arm] + rig.meshes:
        obj.matrix_world = rz @ obj.matrix_world
    update()
    hips = rig.head("hips")
    pts = rig.mesh_points()
    move = Vector((-hips.x, -hips.y, -float(pts[:, 2].min())))
    for obj in [rig.arm] + rig.meshes:
        obj.matrix_world = Matrix.Translation(move) @ obj.matrix_world
    update()
    select_only([rig.arm] + rig.meshes, rig.arm)
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    for obj in rig.meshes:
        obj.parent = rig.arm
        obj.matrix_parent_inverse = Matrix.Identity(4)
    update()
    return math.degrees(turn), to_gltf(move)


def level_arms(rig, elbows):
    """Lifts each arm at the shoulder until upper arm to hand lies level along X."""
    lifted = {}
    for side, x in (("left", 1.0), ("right", -1.0)):
        cap = side.capitalize()
        if elbows == "straighten":
            first = rig.aim(side + "UpperArm", side + "LowerArm", (x, 0.0, 0.0))
            second = rig.aim(side + "LowerArm", side + "Hand", (x, 0.0, 0.0))
            lifted[side] = {"shoulderDeg": first, "elbowDeg": second}
        else:
            lifted[side] = {"shoulderDeg": rig.aim(side + "UpperArm", side + "Hand", (x, 0.0, 0.0))}
        log(f"{cap} arm turned {lifted[side]['shoulderDeg']:.1f} degrees at the shoulder")
    return lifted


# Fingers. Each VRM finger is a chain of three bones (the thumb's metacarpal,
# proximal and distal; the others' proximal, intermediate and distal) whose
# last bone ends at its end bone or its tail.


def finger_roles(side, finger):
    joints = ("Metacarpal", "Proximal", "Distal") if finger == "Thumb" else (
        "Proximal", "Intermediate", "Distal")
    return [side + finger + j for j in joints]


def hand_frame(rig, side):
    """The hand's axis (wrist to the middle knuckle), its palm normal (towards
    the palm) and its thumb side, in world space; `None` without the knuckles."""
    h = rig.head(side + "Hand")
    knuckles = {f: rig.head(side + f + "Proximal") for f in FINGERS[1:]}
    knuckles = {f: k for f, k in knuckles.items() if k is not None}
    first = knuckles.get("Index")
    last = knuckles.get("Little") or knuckles.get("Ring")
    if h is None or first is None or last is None:
        return None
    mid = knuckles.get("Middle") or (first + last) / 2.0
    a = mid - h
    across = first - last
    if a.length < 1e-9 or across.length < 1e-9:
        return None
    a = a.normalized()
    # The left hand's axis crossed with little to index points to the palm.
    n = a.cross(across) * (1.0 if side == "left" else -1.0)
    if n.length < 1e-9:
        return None
    n = n.normalized()
    f = (across - a * across.dot(a)).normalized()
    return h, a, n, f


def bunched(rig, side, height):
    """True when the hand has knuckles and every one sits nearer the wrist
    than BUNCHED of the height, as a broken auto rig leaves them."""
    h = rig.head(side + "Hand")
    knuckles = [rig.head(side + f + "Proximal") for f in FINGERS[1:]]
    far = [(k - h).length for k in knuckles if k is not None]
    return h is not None and bool(far) and max(far) < BUNCHED * height


def finger_curls(rig, height):
    """Each finger's curl in degrees: the bends between its bones, and for the
    four fingers how far the first bone dips out of the palm towards it."""
    out = {}
    for side in ("left", "right"):
        if bunched(rig, side, height):
            continue
        frame = hand_frame(rig, side)
        for finger in FINGERS:
            roles = finger_roles(side, finger)
            pts = [rig.head(r) for r in roles]
            if any(p is None for p in pts):
                continue
            pts.append(rig.tip(roles[-1]))
            segs = [pts[i + 1] - pts[i] for i in range(3)]
            if any(s.length < 1e-9 for s in segs):
                continue
            curl = sum(math.degrees(segs[i].angle(segs[i + 1])) for i in range(2))
            if finger != "Thumb" and frame is not None:
                dip = segs[0].normalized().dot(frame[2])
                curl += max(0.0, math.degrees(math.asin(max(-1.0, min(1.0, dip)))))
            out[side + finger] = curl
    return out


def finger_edges(rig, side):
    """Per skinned mesh, the edges whose ends both follow the side's fingers."""
    names = {rig.roles[r] for f in FINGERS for r in finger_roles(side, f) if r in rig.roles}
    out = []
    for obj in rig.meshes:
        groups = {g.index for g in obj.vertex_groups if g.name in names}
        if not groups:
            continue
        held = np.zeros(len(obj.data.vertices))
        for v in obj.data.vertices:
            held[v.index] = sum(g.weight for g in v.groups if g.group in groups)
        edges = np.empty(len(obj.data.edges) * 2, np.int64)
        obj.data.edges.foreach_get("vertices", edges)
        edges = edges.reshape(-1, 2)
        keep = (held[edges[:, 0]] > FINGER_WEIGHT) & (held[edges[:, 1]] > FINGER_WEIGHT)
        if keep.any():
            out.append((obj, edges[keep]))
    return out


def edge_lengths(edges):
    """The edges' lengths as deformed now."""
    depsgraph = bpy.context.evaluated_depsgraph_get()
    out = []
    for obj, pairs in edges:
        ev = obj.evaluated_get(depsgraph)
        me = ev.to_mesh()
        co = np.empty(len(me.vertices) * 3, np.float64)
        me.vertices.foreach_get("co", co)
        co = co.reshape(-1, 3)
        ev.to_mesh_clear()
        out.append(np.linalg.norm(co[pairs[:, 0]] - co[pairs[:, 1]], axis=1))
    return np.concatenate(out) if out else np.zeros(0)


def stretch(before, after):
    """The 95th percentile of how much each edge stretched or shrank, a ratio."""
    ok = before > 1e-9
    if not ok.any():
        return 1.0
    q = after[ok] / before[ok]
    return float(np.percentile(np.maximum(q, 1.0 / np.maximum(q, 1e-9)), 95))


def finger_targets(rig, side, frame):
    """Where each finger points once straight: the four along the hand, keeping
    part of the knuckles' fan, the thumb 45 degrees between hand and front."""
    h, a, n, f = frame
    out = {}
    for finger in FINGERS:
        roles = finger_roles(side, finger)
        if any(rig.pose_bone(r) is None for r in roles):
            continue
        if finger == "Thumb":
            out[finger] = (a + f).normalized()
            continue
        fan = rig.head(roles[0]) - h
        fan -= n * fan.dot(n)
        yaw = math.atan2(fan.dot(f), fan.dot(a)) * SPREAD_KEEP if fan.length > 1e-9 else 0.0
        out[finger] = a * math.cos(yaw) + f * math.sin(yaw)
    return out


def straighten_hand(rig, side, targets, share):
    turned = 0.0
    for finger, target in targets.items():
        for role in finger_roles(side, finger):
            turned = max(turned, rig.aim_chain(role, target, share))
    return turned


def straighten_fingers(rig):
    """Lays each hand's fingers straight, as far as its mesh holds. Returns per
    side the share of the way it went, the stretch there, the largest joint
    turn and, for a hand left as it was, why."""
    height = float(np.ptp(rig.mesh_points()[:, 2]))
    out = {}
    for side in ("left", "right"):
        if bunched(rig, side, height):
            out[side] = {"share": 0.0, "skipped": "the finger bones sit bunched at the wrist"}
            log(f"{side.capitalize()} hand: finger bones bunched at the wrist, left as they are")
            continue
        frame = hand_frame(rig, side)
        if frame is None:
            out[side] = {"share": 0.0, "skipped": "the hand has no knuckles to read"}
            continue
        targets = finger_targets(rig, side, frame)
        bones = [rig.pose_bone(r) for f in targets for r in finger_roles(side, f)]
        kept = [pb.matrix_basis.copy() for pb in bones]
        edges = finger_edges(rig, side)
        rest = edge_lengths(edges)
        result = {"share": 0.0, "skipped": "every share tore the fingers"}
        for share in SHARES:
            for pb, m in zip(bones, kept):
                pb.matrix_basis = m.copy()
            update()
            turned = straighten_hand(rig, side, targets, share)
            s = stretch(rest, edge_lengths(edges))
            if s <= STRETCH_MAX:
                result = {"share": share, "stretch": s, "turnedDeg": turned}
                break
        else:
            for pb, m in zip(bones, kept):
                pb.matrix_basis = m.copy()
            update()
        out[side] = result
        log(f"{side.capitalize()} fingers straightened {result['share'] * 100:.0f}% of the way")
    return out


# Repairing bunched fingers. A broken auto rig can leave a hand's finger
# joints within a centimetre or two of the wrist, with the finger mesh
# weighted to the hand or the forearm, so the fingers can never open. The
# mesh still has the fingers: the hand's surface is what the mesh edges reach
# from the wrist, each finger tip a top of the surface distance from the wrist
# that stands out from the surface around it, and the thumb the finger that
# joins the hand nearest the wrist. Each finger's joints go along its tube and
# its vertices are weighted to them.


def surface(rig):
    """Every skinned mesh at rest as one welded surface, in world space: each
    vertex's welded vertex, the welded positions, edges and adjacency, and
    which mesh each vertex belongs to."""
    pos, edges, parts = [], [], []
    start = 0
    for obj in rig.meshes:
        me = obj.data
        co = np.empty(len(me.vertices) * 3, np.float64)
        me.vertices.foreach_get("co", co)
        mw = np.array(obj.matrix_world)
        pos.append(co.reshape(-1, 3) @ mw[:3, :3].T + mw[:3, 3])
        e = np.empty(len(me.edges) * 2, np.int64)
        me.edges.foreach_get("vertices", e)
        edges.append(e.reshape(-1, 2) + start)
        parts.append((obj, start))
        start += len(me.vertices)
    pos = np.concatenate(pos)
    height = float(np.ptp(pos[:, 2]))
    _, of = np.unique(np.round(pos / (1e-5 * height)).astype(np.int64), axis=0,
                      return_inverse=True)
    of = of.ravel()
    welded = np.zeros((int(of.max()) + 1, 3))
    welded[of] = pos
    e = np.sort(of[np.concatenate(edges)], 1)
    e = np.unique(e[e[:, 0] != e[:, 1]], axis=0)
    adj = [[] for _ in range(len(welded))]
    for (a, b), l in zip(e.tolist(), np.linalg.norm(welded[e[:, 0]] - welded[e[:, 1]],
                                                    axis=1).tolist()):
        adj[a].append((b, l))
        adj[b].append((a, l))
    return {"pos": pos, "of": of, "welded": welded, "adj": adj, "parts": parts,
            "height": height}


def spread(adj, sources, allowed):
    """Surface distance from the sources (a list of welded vertices) through
    allowed vertices, and the index of the nearest source (-1 unreached)."""
    dist = np.full(len(adj), np.inf)
    near = np.full(len(adj), -1)
    heap = []
    for k, s in enumerate(sources):
        dist[s] = 0.0
        near[s] = k
        heap.append((0.0, s))
    heapq.heapify(heap)
    while heap:
        d, u = heapq.heappop(heap)
        if d > dist[u]:
            continue
        for v, l in adj[u]:
            if allowed[v] and d + l < dist[v]:
                dist[v] = d + l
                near[v] = near[u]
                heapq.heappush(heap, (d + l, v))
    return dist, near


def finger_tips(adj, dist, region):
    """The tops of the surface distance from the wrist, as (vertex, distance,
    how far it stands out, the distance where it joins a taller top): the
    surface is joined from the top down, and where two parts meet the lower
    top dies. A top that never meets a taller one stands out its whole height
    and joins at infinity."""
    parent = {}
    top = {}

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    out = []
    order = [int(v) for v in np.argsort(-dist) if region[v]]
    for u in order:
        parent[u] = u
        top[u] = u
        for v, _ in adj[u]:
            if v not in parent:
                continue
            ru, rv = find(u), find(v)
            if ru == rv:
                continue
            if dist[top[ru]] < dist[top[rv]]:
                ru, rv = rv, ru
            out.append((top[rv], float(dist[top[rv]]), float(dist[top[rv]] - dist[u]),
                        float(dist[u])))
            parent[rv] = ru
    for r in {find(u) for u in order}:
        out.append((top[r], float(dist[top[r]]), float(dist[top[r]]), math.inf))
    return out


def read_weights(rig):
    """Each vertex's bone weights, {bone name: weight}, over every skinned mesh."""
    bones = set(rig.arm.data.bones.keys())
    out = []
    for obj in rig.meshes:
        names = {g.index: g.name for g in obj.vertex_groups}
        for v in obj.data.vertices:
            out.append({names[g.group]: g.weight for g in v.groups
                        if names.get(g.group) in bones and g.weight > 0.0})
    return out


def find_fingers(rig, side, shape):
    """The hand's surface and its five fingers, or None when the mesh does not
    show five: per finger its welded vertices, their surface distance from the
    wrist and its joints (four points, the last its tip)."""
    if any(r not in rig.roles for f in FINGERS for r in finger_roles(side, f)):
        return None
    h = shape["height"]
    welded, adj = shape["welded"], shape["adj"]
    wrist = np.array(rig.head(side + "Hand"))
    axis = wrist - np.array(rig.head(side + "LowerArm"))
    axis /= np.linalg.norm(axis)
    rel = welded - wrist
    along = rel @ axis
    across = np.linalg.norm(rel - np.outer(along, axis), axis=1)
    near = (along > -REPAIR_WRIST * h) & (np.linalg.norm(rel, axis=1) < REPAIR_REACH * h)
    hand_bones = {rig.roles[r] for r in (side + "LowerArm", side + "Hand")}
    hand_bones |= {rig.roles[r] for f in FINGERS for r in finger_roles(side, f) if r in rig.roles}
    held = np.zeros(len(welded))
    np.maximum.at(held, shape["of"], np.array([sum(w for b, w in ws.items() if b in hand_bones)
                                               for ws in shape["weights"]]))
    seeds = near & (np.abs(along) < REPAIR_WRIST * h) & (across < REPAIR_RING * h) & (held > 0.5)
    if not seeds.any():
        return None
    dist, _ = spread(adj, np.nonzero(seeds)[0].tolist(), near)
    region = np.isfinite(dist)
    tips = [t for t in finger_tips(adj, dist, region)
            if t[1] >= REPAIR_TIP * h and t[2] >= REPAIR_KEEP * h]
    tips.sort(key=lambda t: -t[2])
    # Five tips, and a sixth (a nail, a fold) clearly below them.
    if len(tips) < 5 or (len(tips) > 5 and tips[5][2] > 0.5 * tips[4][2]):
        return None
    tips = tips[:5]
    to_tip = np.array([spread(adj, [t[0]], region)[0] for t in tips])
    cell = np.where(region, to_tip.argmin(0), -1)
    thumb = min(range(5), key=lambda k: tips[k][3])
    fingers = []
    for k, (tip, reach, _, _) in enumerate(tips):
        levels = THUMB_LEVELS if k == thumb else FINGER_LEVELS
        mine = np.nonzero(cell == k)[0]
        # A sparse mesh may have no vertex at a joint's distance: each joint
        # is the finger's surface weighted by how near it lies to that distance.
        width = max(REPAIR_SLICE * reach, 0.002 * h)
        joints = []
        for level in levels[:-1]:
            near_level = np.exp(-(((dist[mine] - level * reach) / width) ** 2))
            if near_level.sum() < 1e-6:
                return None
            joints.append((welded[mine] * near_level[:, None]).sum(0) / near_level.sum())
        joints.append(welded[tip])
        fingers.append({"vertices": mine, "joints": joints, "tip": k, "thumb": k == thumb})
    # The four fingers in order from the thumb: along the line their knuckles make.
    thumb_at = next(f["joints"][0] for f in fingers if f["thumb"])
    four = [f for f in fingers if not f["thumb"]]
    knuckles = np.array([f["joints"][0] for f in four])
    centre = knuckles.mean(0)
    line = np.linalg.svd(knuckles - centre)[2][0]
    if (thumb_at - centre) @ line < 0:
        line = -line
    four.sort(key=lambda f: -float((f["joints"][0] - centre) @ line))
    named = {"Thumb": next(f for f in fingers if f["thumb"])}
    named.update(zip(FINGERS[1:], four))
    return {"dist": dist, "region": region, "toTip": to_tip, "fingers": named, "wrist": wrist}


def place_joints(rig, side, fingers):
    """Moves each finger's joints, at rest, to the points found along it; the
    last bone's end bone (the tip joint) goes to the tip."""
    select_only([rig.arm], rig.arm)
    bpy.ops.object.mode_set(mode="EDIT")
    inv = rig.arm.matrix_world.inverted()
    edit = rig.arm.data.edit_bones
    for finger, found in fingers.items():
        names = [rig.roles[r] for r in finger_roles(side, finger)]
        ends = [c.name for c in rig.arm.data.bones[names[-1]].children]
        pts = [inv @ Vector(p) for p in found["joints"]]
        for k, name in enumerate(names):
            edit[name].use_connect = False
            edit[name].head = pts[k]
            edit[name].tail = pts[k + 1]
        for name in ends:
            edit[name].use_connect = False
            edit[name].head = pts[3]
            edit[name].tail = pts[3] + (pts[3] - pts[2]) * 0.5
    bpy.ops.object.mode_set(mode="OBJECT")
    update()


def finger_share(p, wrist, joints):
    """Where along its finger a point lies, as a bone index with a fraction:
    0 is the hand, 1 to 3 the finger's bones, read from the nearest point on
    the line wrist, joints, tip and blended over a band around each joint so
    the skin bends smoothly."""
    pts = [np.asarray(wrist)] + [np.asarray(j) for j in joints]
    lengths = [float(np.linalg.norm(b - a)) for a, b in zip(pts[:-1], pts[1:])]
    arcs = np.concatenate([[0.0], np.cumsum(lengths)])
    best, at = math.inf, 0.0
    for k, (a, b) in enumerate(zip(pts[:-1], pts[1:])):
        ab = b - a
        t = float(np.clip((p - a) @ ab / max(float(ab @ ab), 1e-12), 0.0, 1.0))
        gap = float(np.linalg.norm(p - a - t * ab))
        if gap < best:
            best, at = gap, arcs[k] + t * lengths[k]
    x = 0.0
    for k in range(3):
        half = max(REPAIR_FADE * min(lengths[k], lengths[k + 1]), 1e-6)
        x += float(np.clip((at - arcs[k + 1] + half) / (2.0 * half), 0.0, 1.0))
    return x


def finger_weights(rig, side, finger, x):
    """The bone weights at bone index `x` along `finger` (see finger_share)."""
    chain = [rig.roles[side + "Hand"]] + [rig.roles[r] for r in finger_roles(side, finger)]
    lo = int(min(math.floor(x), 3))
    out = {chain[lo]: 1.0 - (x - lo)}
    if lo < 3 and x > lo:
        out[chain[lo + 1]] = x - lo
    return out


def reweight_hand(rig, side, shape, found):
    """Weights the hand's surface to its new finger joints, fading from the old
    weights at the wrist; finger weight left anywhere else goes to the hand.
    Small pieces of their own lying on a finger (a ring) take the weights of
    the finger surface nearest them. Returns how many vertices changed."""
    h = shape["height"]
    of, dist = shape["of"], found["dist"]
    hand = rig.roles[side + "Hand"]
    finger_bones = {rig.roles[r] for f in FINGERS for r in finger_roles(side, f) if r in rig.roles}
    for f in FINGERS:
        for c in rig.arm.data.bones[rig.roles[finger_roles(side, f)[-1]]].children:
            finger_bones.add(c.name)
    by_tip = {info["tip"]: f for f, info in found["fingers"].items()}
    of_finger = {}
    for f, info in found["fingers"].items():
        for v in info["vertices"]:
            of_finger[int(v)] = f
    # Beside the seam between two fingers the skin blends into the other one,
    # so fingers the mesh fuses stretch the seam smoothly instead of tearing it.
    to_tip = np.where(np.isfinite(found["toTip"]), found["toTip"], 1e9)
    order = np.argsort(to_tip, axis=0)
    gap = np.take_along_axis(to_tip, order[1:2], 0)[0] - np.take_along_axis(to_tip, order[:1], 0)[0]
    other = 0.5 * smoothstep(1.0 - gap / (REPAIR_MIX * h))
    pos = shape["pos"]
    out = []
    for i, old in enumerate(shape["weights"]):
        ws = {}
        for b, w in old.items():
            b = hand if b in finger_bones else b
            ws[b] = ws.get(b, 0.0) + w
        w = int(of[i])
        f = of_finger.get(w)
        if f is not None:
            new = {}
            second = by_tip[int(order[1, w])]
            for g, share in ((f, 1.0 - other[w]), (second, other[w])):
                if share <= 0.0:
                    continue
                x = finger_share(pos[i], found["wrist"], found["fingers"][g]["joints"])
                for b, bw in finger_weights(rig, side, g, x).items():
                    new[b] = new.get(b, 0.0) + share * bw
            s = float(smoothstep(float(dist[w]) / (REPAIR_BLEND * h)))
            mixed = {b: (1.0 - s) * bw for b, bw in ws.items()}
            for b, bw in new.items():
                mixed[b] = mixed.get(b, 0.0) + s * bw
            ws = mixed
        out.append(ws)
    # A ring or a nail of its own: most of it within reach of a finger's surface.
    welded = shape["welded"]
    on = np.zeros(len(welded), bool)
    on[list(of_finger)] = True
    surface_at = welded[on]
    surface_of = np.nonzero(on)[0]
    rows = {}
    for i, w in enumerate(of.tolist()):
        rows.setdefault(w, []).append(i)
    wrist = np.array(rig.head(side + "Hand"))
    for members in islands(shape):
        if on[members].any() or len(members) > REPAIR_PIECE * len(welded):
            continue
        pts = welded[members]
        if np.linalg.norm(pts - wrist, axis=1).max() > REPAIR_REACH * h:
            continue
        gap = np.linalg.norm(pts[:, None, :] - surface_at[None], axis=2)
        if (gap.min(1) <= REPAIR_TOUCH * h).mean() < 0.7:
            continue
        nearest = surface_of[gap.argmin(1)]
        for w, take in zip(members.tolist(), nearest.tolist()):
            for i in rows[w]:
                out[i] = dict(out[rows[take][0]])
    changed = 0
    for i, (old, ws) in enumerate(zip(shape["weights"], out)):
        top = sorted(ws.items(), key=lambda bw: -bw[1])[:MAX_INFLUENCES]
        total = sum(w for _, w in top)
        top = {b: w / total for b, w in top if w > 0.0}
        if top.keys() != old.keys() or any(abs(top[b] - old[b]) > 1e-4 for b in top):
            out[i] = top
            changed += 1
        else:
            out[i] = None
    for obj, start in shape["parts"]:
        groups = obj.vertex_groups
        for k in range(len(obj.data.vertices)):
            ws = out[start + k]
            if ws is None:
                continue
            # Indices first: removing a group shifts the vertex's own list.
            for gi in [g.group for g in obj.data.vertices[k].groups]:
                if groups[gi].name in rig.arm.data.bones:
                    groups[gi].remove([k])
            for b, w in ws.items():
                group = groups.get(b) or groups.new(name=b)
                group.add([k], w, "REPLACE")
    return changed


def islands(shape):
    """The welded surface's pieces of their own, as arrays of welded vertices."""
    adj = shape["adj"]
    label = np.full(len(adj), -1)
    out = []
    for s in range(len(adj)):
        if label[s] >= 0:
            continue
        label[s] = len(out)
        stack, members = [s], [s]
        while stack:
            u = stack.pop()
            for v, _ in adj[u]:
                if label[v] < 0:
                    label[v] = len(out)
                    stack.append(v)
                    members.append(v)
        out.append(np.array(members))
    return out


def smoothstep(x):
    x = np.clip(x, 0.0, 1.0)
    return x * x * (3.0 - 2.0 * x)


def repair_hands(rig):
    """Places the finger joints of each hand whose finger bones sit bunched at
    the wrist along the fingers its mesh shows, and weights the fingers to
    them. Returns per repaired side how many vertices were reweighted and the
    middle finger's reach from the wrist before and after; a hand whose mesh
    does not show five fingers is left as it is."""
    update()
    shape = None
    out = {}
    for side in ("left", "right"):
        if shape is None:
            shape = surface(rig)
            height = shape["height"]
        if not bunched(rig, side, height):
            continue
        if shape.get("weights") is None:
            shape["weights"] = read_weights(rig)
        found = find_fingers(rig, side, shape)
        if found is None:
            log(f"{side.capitalize()} hand: finger bones bunched at the wrist and five fingers not found")
            continue
        before = hand_reach(rig, side)
        place_joints(rig, side, found["fingers"])
        changed = reweight_hand(rig, side, shape, found)
        shape["weights"] = read_weights(rig)
        out[side] = {"vertices": changed, "reachBefore": before, "reachAfter": hand_reach(rig, side)}
        log(f"{side.capitalize()} hand: finger joints placed along its fingers, "
            f"{changed} vertices reweighted")
    return out


def apply_as_rest(rig):
    """Bakes the current pose into the meshes and makes it the armature's rest."""
    kept = {}
    for obj in rig.meshes:
        mod = next(m for m in obj.modifiers if m.type == "ARMATURE" and m.object == rig.arm)
        kept[obj.name] = {
            "name": mod.name,
            "use_vertex_groups": mod.use_vertex_groups,
            "use_bone_envelopes": mod.use_bone_envelopes,
            "use_deform_preserve_volume": mod.use_deform_preserve_volume,
            "use_multi_modifier": mod.use_multi_modifier,
        }
        select_only([obj], obj)
        bpy.ops.object.modifier_apply(modifier=mod.name)
    select_only([rig.arm], rig.arm)
    bpy.ops.object.mode_set(mode="POSE")
    bpy.ops.pose.armature_apply(selected=False)
    bpy.ops.object.mode_set(mode="OBJECT")
    for obj in rig.meshes:
        k = kept[obj.name]
        mod = obj.modifiers.new(k.pop("name"), "ARMATURE")
        mod.object = rig.arm
        for key, value in k.items():
            setattr(mod, key, value)
        # The armature modifier deforms first, as it did.
        at = list(obj.modifiers).index(mod)
        if at:
            obj.modifiers.move(at, 0)
    update()


def weight_checks(rig, fix):
    """Influences per vertex and weight sums over the bone groups; with `fix`,
    trims to the top 4 and normalises the vertices that need it."""
    bones = set(rig.arm.data.bones.keys())
    out = {"vertices": 0, "overInfluences": 0, "offSum": 0, "unweighted": 0, "maxInfluences": 0,
           "minSum": None, "maxSum": None, "fixed": 0}
    for obj in rig.meshes:
        names = {g.index: g.name for g in obj.vertex_groups}
        groups = obj.vertex_groups
        for v in obj.data.vertices:
            ws = [(g.group, g.weight) for g in v.groups if names.get(g.group) in bones and g.weight > 0.0]
            out["vertices"] += 1
            n = len(ws)
            out["maxInfluences"] = max(out["maxInfluences"], n)
            total = sum(w for _, w in ws)
            if n == 0:
                out["unweighted"] += 1
                continue
            out["minSum"] = total if out["minSum"] is None else min(out["minSum"], total)
            out["maxSum"] = total if out["maxSum"] is None else max(out["maxSum"], total)
            over = n > MAX_INFLUENCES
            off = abs(total - 1.0) > 1e-3
            out["overInfluences"] += over
            out["offSum"] += off
            if fix and (over or off):
                ws.sort(key=lambda gw: -gw[1])
                keep = ws[:MAX_INFLUENCES]
                s = sum(w for _, w in keep)
                for gi, _ in ws[MAX_INFLUENCES:]:
                    groups[gi].remove([v.index])
                for gi, w in keep:
                    groups[gi].add([v.index], w / s, "REPLACE")
                out["fixed"] += 1
    return out


# Pose sheets.


def sheet_pose(rig, pose):
    if pose == "rest":
        return
    if pose in ("down", "raise"):
        deg = -DOWN_DEG if pose == "down" else RAISE_DEG
        r = math.radians(deg)
        for side, x in (("left", 1.0), ("right", -1.0)):
            rig.aim(side + "UpperArm", side + "Hand", (x * math.cos(r), 0.0, math.sin(r)))
    elif pose == "step":
        rx = lambda deg: Quaternion(Vector((1, 0, 0)), math.radians(deg))
        rig.world_rotate("leftUpperLeg", rx(-35))
        rig.world_rotate("leftLowerLeg", rx(40))
        rig.world_rotate("rightUpperLeg", rx(25))
    else:
        fail("unknown pose " + pose)


def setup_scene():
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
    sc.camera = cam
    txt = bpy.data.curves.new("sheet_label", "FONT")
    label = bpy.data.objects.new("sheet_label", txt)
    sc.collection.objects.link(label)
    return cam, label


def frame_for(rig):
    """The camera frame for the whole body and for the shoulders, from the rest."""
    pts = rig.mesh_points()
    height = float(pts[:, 2].max() - pts[:, 2].min())
    # The arms' reach when level, so before and after share one frame.
    reach = 0.0
    for side in ("left", "right"):
        u, lo, h = (rig.head(side + b) for b in ("UpperArm", "LowerArm", "Hand"))
        reach = max(reach, abs(u.x) + (lo - u).length + (h - lo).length + 0.12 * height)
    body = max(1.08 * height, 2.1 * reach)
    shoulders = rig.head("leftUpperArm").z
    return {"body": (body, height / 2), "shoulders": (0.5 * height, shoulders - 0.05 * height),
            "height": height}


def place(cam, label, view, frame, text):
    scale, z = frame["shoulders"] if view == "shoulders" else frame["body"]
    yaw = {"front": 0.0, "shoulders": 0.0, "side": 90.0, "back": 180.0}[view]
    y = math.radians(yaw)
    d = Vector((math.sin(y), -math.cos(y), 0.0))
    centre = Vector((0.0, 0.0, z))
    cam.data.ortho_scale = scale
    cam.location = centre + d * 10.0
    cam.rotation_euler = (-d).to_track_quat("-Z", "Y").to_euler()
    cam.data.clip_start = 0.1
    cam.data.clip_end = 30.0
    right = Vector((math.cos(y), math.sin(y), 0.0))
    label.data.body = text
    label.data.size = 0.03 * scale
    label.location = centre + d * 5.0 - right * 0.46 * scale + Vector((0.0, 0.0, 0.44 * scale))
    label.rotation_euler = (math.radians(90.0), 0.0, y)


def hand_reach(rig, side):
    """Wrist to the middle finger's end along its bones, metres; the same in
    any pose, so the before and after crops share one scale."""
    roles = [side + "Hand"] + finger_roles(side, "Middle")
    pts = [rig.head(r) for r in roles]
    if any(p is None for p in pts):
        return None
    pts.append(rig.tip(roles[-1]))
    return sum((pts[i + 1] - pts[i]).length for i in range(len(pts) - 1))


def place_hand(cam, label, rig, side, view, frame, text):
    """Frames one hand at rest from its thumb side ("front") or its back
    ("top"), clipped to the hand so the body around it stays out. False when
    the hand cannot be read."""
    hand = hand_frame(rig, side)
    reach = hand_reach(rig, side)
    if hand is None or reach is None:
        return False
    # A hand whose bones are bunched up still gets a hand sized frame.
    reach = max(reach, HAND_MIN * frame["height"])
    h, a, n, f = hand
    pts = [h]
    for finger in FINGERS:
        for role in finger_roles(side, finger):
            if rig.head(role) is not None:
                pts += [rig.head(role), rig.tip(role)]
    lo = Vector([min(p[i] for p in pts) for i in range(3)])
    hi = Vector([max(p[i] for p in pts) for i in range(3)])
    centre = (lo + hi) / 2.0
    back, up = (f, -n) if view == "front" else (-n, -f)
    right = up.cross(back)
    rot = Matrix((right, up, back)).transposed()
    scale = 1.35 * reach
    dist = 3.0 * reach
    cam.data.ortho_scale = scale
    cam.location = centre + back * dist
    cam.rotation_euler = rot.to_euler()
    cam.data.clip_start = dist - 0.5 * reach
    cam.data.clip_end = dist + 0.5 * reach
    label.data.body = text
    label.data.size = 0.03 * scale
    label.location = centre + back * 0.5 * reach - right * 0.46 * scale + up * 0.44 * scale
    label.rotation_euler = rot.to_euler()
    return True


def render_cell(path):
    bpy.context.scene.render.filepath = path
    bpy.ops.render.render(write_still=True)
    img = bpy.data.images.load(path)
    px = np.array(img.pixels[:], np.float32).reshape(CELL, CELL, 4)[::-1]
    bpy.data.images.remove(img)
    return px


def render_sheet(rig, out, title, frame, repaired=()):
    cam, label = setup_scene()
    cells_dir = out + "_cells"
    os.makedirs(cells_dir, exist_ok=True)
    rows = len(SHEET_POSES) + 1
    sheet = np.ones((CELL * rows, CELL * len(SHEET_VIEWS), 4), np.float32)
    for ri, pose in enumerate(SHEET_POSES):
        rig.reset_pose()
        sheet_pose(rig, pose)
        for ci, view in enumerate(SHEET_VIEWS):
            place(cam, label, view, frame, f"{title}  {pose}  {view}")
            px = render_cell(os.path.join(cells_dir, f"{pose}_{view}.png"))
            sheet[ri * CELL:(ri + 1) * CELL, ci * CELL:(ci + 1) * CELL] = px
    rig.reset_pose()
    ri = len(SHEET_POSES)
    for ci, (side, view) in enumerate(HAND_VIEWS):
        cell = sheet[ri * CELL:(ri + 1) * CELL, ci * CELL:(ci + 1) * CELL]
        text = f"{title}  {side} hand  {view}" + ("  joints repaired" if side in repaired else "")
        if place_hand(cam, label, rig, side, view, frame, text):
            cell[:] = render_cell(os.path.join(cells_dir, f"hand_{side}_{view}.png"))
        else:
            cell[:, :, :3] = 0.4
    for ri in range(1, rows):
        sheet[ri * CELL:ri * CELL + 2, :, :3] = 0.15
    for ci in range(1, len(SHEET_VIEWS)):
        sheet[:, ci * CELL:ci * CELL + 2, :3] = 0.15
    bpy.data.objects.remove(cam)
    bpy.data.objects.remove(label)
    img = bpy.data.images.new("sheet", sheet.shape[1], sheet.shape[0], alpha=True)
    img.pixels.foreach_set(sheet[::-1].ravel())
    img.filepath_raw = out
    img.file_format = "PNG"
    img.save()
    bpy.data.images.remove(img)
    for name in os.listdir(cells_dir):
        os.remove(os.path.join(cells_dir, name))
    os.rmdir(cells_dir)
    log(f"sheet {out}")


def export(rig, out):
    select_only([rig.arm] + rig.meshes, rig.arm)
    bpy.ops.export_scene.gltf(
        filepath=out,
        export_format="GLB",
        use_selection=True,
        export_animations=False,
        export_skins=True,
        export_yup=True,
    )


def main():
    args = parse_args()
    job = load_job(args.job)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=job["input"])
    rig = Rig(job["bones"])
    rig.reset_pose()
    before = measure(rig)
    turned, moved = orient(rig)
    log(f"turned {turned:.1f} degrees, moved {moved}")
    frame = frame_for(rig)
    if "before" in job["sheets"]:
        render_sheet(rig, job["sheets"]["before"], "before", frame)
    weights_before = weight_checks(rig, fix=False)
    repaired = repair_hands(rig)
    lifted = level_arms(rig, job["elbows"])
    straightened = straighten_fingers(rig) if job["fingers"] == "straighten" else {}
    apply_as_rest(rig)
    weights = weight_checks(rig, fix=True)
    rig.reset_pose()
    after = measure(rig)
    if "after" in job["sheets"]:
        render_sheet(rig, job["sheets"]["after"], "after", frame, repaired)
    export(rig, job["output"])
    report = {
        "before": before,
        "after": after,
        "turnedDeg": turned,
        "moved": moved,
        "elbows": job["elbows"],
        "lifted": lifted,
        "fingers": job["fingers"],
        "straightened": straightened,
        "repaired": repaired,
        "weightsBefore": weights_before,
        "weights": weights,
    }
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
        fail("normalising stopped: " + traceback.format_exc().strip().splitlines()[-1])
