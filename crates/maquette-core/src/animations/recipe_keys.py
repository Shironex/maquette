"""Pose keys for the recipe bake: each keyed bone blended between its keys
at a frame's share, then laid on the pose as it stands.

bake_recipe.py imports this from beside it. The job holds each pose key
change as

    {"mode": "add", "weight": 1.0,
     "keys": [{"share": 0.22, "ease": "out", "pose": {"chest": [0, -18, 0]}}, ...]}

with its keys first to last by share (the core works the shares out from
each key's clip and frame, as the rough preview does). A bone blends only
between the keys that turn it: before its first key it holds that key, after
its last the last, and between two it moves as the later key's ease says
(smooth, in, out, linear, or step, which holds the key before until its own
frame).

A turn is degrees about the model's own axes (X to its left, Y up, Z ahead),
X then Y then Z, as the bone stands under its parent: with its parent's turn
from rest P, add turns the bone by P (B K B^-1) P^-1 on top of the clip, and
replace sets its world rotation to P (B K B^-1) R, R its rest. B turns the
model's axes to the world's, so a key reads the same on any rig. Parents go
before their children.
"""

import math

import bpy
from mathutils import Matrix, Quaternion

EPS = 1e-9


def ease(kind, t):
    t = min(max(t, 0.0), 1.0)
    if kind == "in":
        return t * t
    if kind == "out":
        return 1.0 - (1.0 - t) * (1.0 - t)
    if kind == "linear":
        return t
    if kind == "step":
        return 1.0 if t >= 1.0 - EPS else 0.0
    return t * t * (3.0 - 2.0 * t)


def sample(track, share):
    """A bone's turn at `share` from its keys, each (share, ease, turn)."""
    i = None
    for k, point in enumerate(track):
        if point[0] <= share + EPS:
            i = k
    if i is None:
        return list(track[0][2])
    if i + 1 >= len(track):
        return list(track[i][2])
    a, b = track[i], track[i + 1]
    span = b[0] - a[0]
    t = (share - a[0]) / span if span > EPS else 1.0
    e = ease(b[1], t)
    return [a[2][k] + (b[2][k] - a[2][k]) * e for k in range(3)]


def turns_at(keys, share):
    """Every keyed bone's turn at `share`, degrees, by VRM name."""
    tracks = {}
    for key in keys:
        for bone, turn in key["pose"].items():
            tracks.setdefault(bone, []).append((float(key["share"]), key.get("ease", "smooth"), [float(v) for v in turn]))
    return {bone: sample(track, share) for bone, track in tracks.items()}


def turn_of(degrees):
    x, y, z = (math.radians(v) for v in degrees)
    return (Matrix.Rotation(x, 3, "X") @ Matrix.Rotation(y, 3, "Y") @ Matrix.Rotation(z, 3, "Z")).to_quaternion()


def depth(pb):
    n = 0
    while pb.parent is not None:
        n += 1
        pb = pb.parent
    return n


def rest_rotation(rig, pb):
    return (rig.mw @ pb.bone.matrix_local).to_3x3().normalized().to_quaternion()


def parent_turn(rig, pb):
    """How far the bone's parent is turned from its rest, in the world."""
    if pb.parent is None:
        return Quaternion()
    return rig.world_rotation(pb.parent) @ rest_rotation(rig, pb.parent).inverted()


def lay(rig, spec, share):
    """Lays one pose key change at `share` over the pose as it stands."""
    if not spec or not spec.get("keys"):
        return
    weight = min(max(float(spec.get("weight", 1.0)), 0.0), 1.0)
    if weight <= 0.0:
        return
    replace = spec.get("mode") == "replace"
    bones = []
    for vrm, degrees in turns_at(spec["keys"], share).items():
        pb = rig.pose_bone(vrm)
        if pb is not None:
            bones.append((pb, degrees))
    bones.sort(key=lambda item: depth(item[0]))
    for pb, degrees in bones:
        parent = parent_turn(rig, pb)
        if replace:
            want = parent @ rig.model_turn(turn_of(degrees)) @ rest_rotation(rig, pb)
            now = rig.world_rotation(pb)
            if now.dot(want) < 0.0:
                want.negate()
            step = now.slerp(want, weight) @ now.inverted()
        else:
            added = rig.model_turn(Quaternion().slerp(turn_of(degrees), weight))
            step = parent @ added @ parent.inverted()
        rig.turn(pb, step)
        bpy.context.view_layer.update()
