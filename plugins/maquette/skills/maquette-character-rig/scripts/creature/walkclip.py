# walkclip.py: procedural, loopable in-place walk for a many-legged creature (30 fps, linear keys), keyframed headless.
# Method: each leg chain gets a phase; stance half = hip yaws from front to back on the ground, swing half
# = hip yaws back to front while lifting (sin arc) with a small knee tuck. Legs are split into two
# alternating sets (generalised tripod gait, see LEGS in rigconfig.py). A leg that hangs under the body
# (posing.leg_axis) swings fore and aft about the creature's right axis instead of yawing. Rotations are world-axis
# about each bone head (posing.rot_world), then keyed as rotation_quaternion every frame, so frame N == 0.
# Tables (LEGS, SPIKES, HEAD_BONE, YAW, LIFT, TUCK) come from rigconfig.py. Spikes get a tiny out-of-phase shimmer, head a small sway. Usage: -- in.glb out.glb [frames=30]
import bpy, sys, math; sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(sys.argv[sys.argv.index("--")-1])))
from shlib import *
import posing
from posing import rot_world
from mathutils import Vector
a = sys.argv[sys.argv.index("--")+1:]; glb, out = a[:2]; F = int(a[2]) if len(a) > 2 else 30
arm, me, stray = load(glb); posing.bind(arm); mw = arm.matrix_world
pb = arm.pose.bones
for b in pb: b.rotation_mode = "QUATERNION"
act = bpy.data.actions.new("walk"); arm.animation_data_create(); arm.animation_data.action = act
sc = bpy.context.scene; sc.render.fps = 30; sc.frame_start = 0; sc.frame_end = F
keyed = set()
bpy.context.preferences.edit.keyframe_new_interpolation_type = "LINEAR"   # Blender 5 actions have no act.fcurves
for fr in range(F + 1):
    posing.reset()
    t = (fr % F) / F
    for leg, (bs, off) in LEGS.items():
        h = mw @ arm.data.bones[bs[0]].head_local; tl = mw @ arm.data.bones[bs[-1]].tail_local
        d = tl - h; hd = Vector((d.x, d.y, 0)).normalized(); side = hd.cross(UP)
        p = (t + off) % 1.0
        if p < 0.5: yaw = YAW * (1 - 4 * p); lift = 0; tuck = 0          # stance: front -> back
        else:
            s = (p - 0.5) / 0.5; yaw = YAW * (-1 + 2 * (0.5 - 0.5 * math.cos(math.pi * s)))  # swing: back -> front, eased
            lift = LIFT * math.sin(math.pi * s); tuck = TUCK * math.sin(math.pi * s)
        hang = posing.leg_axis(bs)
        if hang is not None:
            # a leg under the body (four legged beast): the hip swings fore and aft about the creature's
            # right axis (positive moves the foot forward), the knee folds during the swing
            rot_world(pb[bs[0]], hang, yaw)
            if tuck: rot_world(pb[bs[1]], hang, 2 * tuck)   # TUCK < 0 folds the lower leg back
        else:
            fwd = 1 if UP.cross(hd).dot(FRONT) > 0 else -1   # +Z yaw sign that moves the tip toward FRONT
            rot_world(pb[bs[0]], UP, fwd * yaw)
            if lift: rot_world(pb[bs[0]], side, lift); rot_world(pb[bs[1]], side, tuck)
        keyed.update(bs)
    rot_world(pb[HEAD_BONE], UP, 3 * math.sin(2 * math.pi * t)); keyed.add(HEAD_BONE)
    for i, sp in enumerate(SPIKES):
        rot_world(pb[sp], LEFT, 2.5 * math.sin(2 * math.pi * (2 * t + i / 3))); keyed.add(sp)
    for n in keyed: pb[n].keyframe_insert("rotation_quaternion", frame=fr)
print("KEYED", len(keyed), "bones", F + 1, "frames")
for s in stray: bpy.data.objects.remove(s, do_unlink=True)
bpy.ops.export_scene.gltf(filepath=out, export_format="GLB", export_image_format="AUTO", export_animations=True,
                          export_animation_mode="ACTIONS", export_skins=True, export_yup=True, export_frame_range=True)
print("EXPORTED", out)
