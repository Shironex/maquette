# posing.py: rig-agnostic chain posing by world-axis rotation about each bone head (no bone-roll knowledge
# needed). bind(arm) first; scale multiplies every angle.
import bpy, math
from mathutils import Vector, Matrix
from shlib import CHAINS, TRUNK_POSES, FRONT, UP, LEFT
arm = None; mw = None; mwi = None
def bind(a):
    global arm, mw, mwi
    arm = a; mw = a.matrix_world.copy(); mwi = mw.inverted()
def reset():
    for pb in arm.pose.bones: pb.matrix_basis = Matrix()
    bpy.context.view_layer.update()
def rot_world(pb, axis, deg):
    bpy.context.view_layer.update()
    head = mw @ pb.head
    R = Matrix.Translation(head) @ Matrix.Rotation(math.radians(deg), 4, axis.normalized()) @ Matrix.Translation(-head)
    pb.matrix = mwi @ R @ mw @ pb.matrix
    bpy.context.view_layer.update()
def chain_dir(bs):
    a = mw @ arm.data.bones[bs[0]].head_local; b = mw @ arm.data.bones[bs[-1]].tail_local
    return a, b
def leg_axis(bs):
    """-LEFT (the creature's right axis) for a leg that hangs under the body, None for a leg that splays out."""
    a, b = chain_dir(bs); d = b - a; hd = Vector((d.x, d.y, 0))
    return None if abs(hd.dot(LEFT)) > 0.5 * d.length else -LEFT
def pose_chain(ch, k=1.0):
    bs = CHAINS[ch]; pb = arm.pose.bones
    a, b = chain_dir(bs); d = b - a; hd = Vector((d.x, d.y, 0))
    if ch.startswith("leg_") or ch.startswith("rear_"):
        # A leg that splays out sideways (spider) bends about the horizontal axis across it; a leg that
        # hangs under the body (four legged beast) has almost no horizontal direction, so it swings
        # fore and aft about the creature's own right axis instead (positive lifts the foot forward).
        side = leg_axis(bs)
        if side is None: side = hd.cross(UP).normalized()
        ang = [30, -25, 15]                               # lift hip, fold knee back down, flick tip
        for i, n in enumerate(bs):
            if i < len(ang): rot_world(pb[n], side, ang[i] * k)
    elif ch in TRUNK_POSES:
        for bone, axis, deg in TRUNK_POSES[ch]: rot_world(pb[bone], Vector(axis), deg * k)
    else:  # cheeks, spikes: tilt sideways/forward
        ax = Vector((d.y, -d.x, 0)) if hd.length > 1e-4 else Vector((1, 0, 0))
        for i, n in enumerate(bs): rot_world(pb[n], ax, (30 if i == 0 else 15) * k)
