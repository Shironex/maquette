"""Tiny rigs and clips for the animation tests. Run headless:

    blender -b --factory-startup --python clip_rigs.py -- <which> <out>

<which> is one of:
- clip: a 22 bone humanoid skeleton with Mixamo style names in a T pose
  and one clip, "Dodge Left" (30 frames at 30 fps, the hips moving 0.9 m
  to the left), written as FBX with a skinned box, the way clip packs come.
- still: the same skeleton holding one pose, with keys that never change.
- slide: the same skeleton with one clip, "Foot Slide" (30 frames), its
  left leg swaying a few degrees so the planted left foot slides about
  5 cm and back, within the leg's reach: a clip Pin the feet can fix.
- custom: a 6 bone chain with names no humanoid rig uses and one clip.
- model: a box figure on a 22 bone humanoid rig with its own bone names
  (hips, spine, chest, upper_arm.L) in an A pose, with one clip "idle",
  written as GLB: the model a library clip is retargeted onto.
"""

import math
import os
import sys

import bpy
from mathutils import Vector

FPS = 30
PREFIX = "mixamorig:"

# name, parent, head, tail in Blender axes (Z up, facing -Y, left is +X).
T_POSE = [
    ("Hips", None, (0, 0, 1.0), (0, 0, 1.1)),
    ("Spine", "Hips", (0, 0, 1.1), (0, 0, 1.2)),
    ("Spine1", "Spine", (0, 0, 1.2), (0, 0, 1.3)),
    ("Spine2", "Spine1", (0, 0, 1.3), (0, 0, 1.42)),
    ("Neck", "Spine2", (0, 0, 1.42), (0, 0, 1.52)),
    ("Head", "Neck", (0, 0, 1.52), (0, 0, 1.72)),
    ("LeftShoulder", "Spine2", (0.05, 0, 1.4), (0.15, 0, 1.4)),
    ("LeftArm", "LeftShoulder", (0.15, 0, 1.4), (0.42, 0, 1.4)),
    ("LeftForeArm", "LeftArm", (0.42, 0, 1.4), (0.66, 0, 1.4)),
    ("LeftHand", "LeftForeArm", (0.66, 0, 1.4), (0.76, 0, 1.4)),
    ("RightShoulder", "Spine2", (-0.05, 0, 1.4), (-0.15, 0, 1.4)),
    ("RightArm", "RightShoulder", (-0.15, 0, 1.4), (-0.42, 0, 1.4)),
    ("RightForeArm", "RightArm", (-0.42, 0, 1.4), (-0.66, 0, 1.4)),
    ("RightHand", "RightForeArm", (-0.66, 0, 1.4), (-0.76, 0, 1.4)),
    ("LeftUpLeg", "Hips", (0.1, 0, 0.98), (0.1, 0, 0.52)),
    ("LeftLeg", "LeftUpLeg", (0.1, 0, 0.52), (0.1, 0, 0.08)),
    ("LeftFoot", "LeftLeg", (0.1, 0, 0.08), (0.1, -0.12, 0.02)),
    ("LeftToeBase", "LeftFoot", (0.1, -0.12, 0.02), (0.1, -0.2, 0.02)),
    ("RightUpLeg", "Hips", (-0.1, 0, 0.98), (-0.1, 0, 0.52)),
    ("RightLeg", "RightUpLeg", (-0.1, 0, 0.52), (-0.1, 0, 0.08)),
    ("RightFoot", "RightLeg", (-0.1, 0, 0.08), (-0.1, -0.12, 0.02)),
    ("RightToeBase", "RightFoot", (-0.1, -0.12, 0.02), (-0.1, -0.2, 0.02)),
]

# The model's own names for the same bones.
MODEL_NAMES = {
    "Hips": "hips",
    "Spine": "spine",
    "Spine1": "spine_01",
    "Spine2": "chest",
    "Neck": "neck",
    "Head": "head",
    "LeftShoulder": "shoulder.L",
    "LeftArm": "upper_arm.L",
    "LeftForeArm": "forearm.L",
    "LeftHand": "hand.L",
    "RightShoulder": "shoulder.R",
    "RightArm": "upper_arm.R",
    "RightForeArm": "forearm.R",
    "RightHand": "hand.R",
    "LeftUpLeg": "thigh.L",
    "LeftLeg": "shin.L",
    "LeftFoot": "foot.L",
    "LeftToeBase": "toe.L",
    "RightUpLeg": "thigh.R",
    "RightLeg": "shin.R",
    "RightFoot": "foot.R",
    "RightToeBase": "toe.R",
}

CHAIN = [
    ("base", None, (0, 0, 0), (0, 0, 0.3)),
    ("seg_a", "base", (0, 0, 0.3), (0, 0, 0.6)),
    ("seg_b", "seg_a", (0, 0, 0.6), (0, 0, 0.9)),
    ("seg_c", "seg_b", (0, 0, 0.9), (0, 0, 1.2)),
    ("seg_d", "seg_c", (0, 0, 1.2), (0, 0, 1.5)),
    ("tip", "seg_d", (0, 0, 1.5), (0, 0, 1.7)),
]


def reset():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.render.fps = FPS
    scene.frame_start = 0
    scene.frame_end = 30
    return scene


def a_pose(bones):
    """Lowers the arms 45 degrees, the way many hand made rigs stand."""
    out = []
    for name, parent, head, tail in bones:
        if name.endswith(("Arm", "ForeArm", "Hand")) and "Shoulder" not in name:
            side = 1 if name.startswith("Left") else -1
            pivot = Vector((0.15 * side, 0, 1.4))
            def lower(p):
                d = Vector(p) - pivot
                c = math.cos(math.radians(45))
                return tuple(pivot + Vector((d.x * c, d.y, -abs(d.x) * c)))
            head, tail = lower(head), lower(tail)
        out.append((name, parent, head, tail))
    return out


def armature(name, bones, rename=lambda n: n):
    data = bpy.data.armatures.new(name)
    obj = bpy.data.objects.new(name, data)
    bpy.context.scene.collection.objects.link(obj)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.mode_set(mode="EDIT")
    made = {}
    for bone, parent, head, tail in bones:
        eb = data.edit_bones.new(rename(bone))
        eb.head, eb.tail = Vector(head), Vector(tail)
        if parent:
            eb.parent = made[parent]
            eb.use_connect = False
        made[bone] = eb
    bpy.ops.object.mode_set(mode="OBJECT")
    return obj


def skinned_box(arm, size=0.3, at=(0, 0, 1.0)):
    bpy.ops.mesh.primitive_cube_add(size=size, location=at)
    box = bpy.context.active_object
    box.name = "body"
    group = box.vertex_groups.new(name=arm.data.bones[0].name)
    group.add(range(len(box.data.vertices)), 1.0, "REPLACE")
    box.modifiers.new("Armature", "ARMATURE").object = arm
    box.parent = arm
    return box


def key_bone(arm, bone, frames, **channels):
    pb = arm.pose.bones[bone]
    pb.rotation_mode = "QUATERNION"
    for frame, values in frames:
        for channel, value in values.items():
            setattr(pb, channel, value)
            pb.keyframe_insert(data_path=channel, frame=frame)


def dodge(arm, hips, arm_bone, still=False):
    arm.animation_data_create().action = bpy.data.actions.new("Dodge Left")
    # The hips point up, so their local X is the world X: a move to the left.
    shift = 0.0 if still else 0.9
    key_bone(arm, hips, [(0, {"location": (0, 0, 0)}), (30, {"location": (shift, 0, 0)})])
    lift = 0.0 if still else 0.4
    key_bone(
        arm,
        arm_bone,
        [(0, {"rotation_quaternion": (1, 0, 0, 0)}), (15, {"rotation_quaternion": (math.cos(lift), 0, 0, math.sin(lift))}), (30, {"rotation_quaternion": (1, 0, 0, 0)})],
    )


def sway(arm, leg):
    arm.animation_data_create().action = bpy.data.actions.new("Foot Slide")
    half = math.radians(3.0) / 2
    key_bone(
        arm,
        leg,
        [(0, {"rotation_quaternion": (1, 0, 0, 0)}), (15, {"rotation_quaternion": (math.cos(half), math.sin(half), 0, 0)}), (30, {"rotation_quaternion": (1, 0, 0, 0)})],
    )
    hips = PREFIX + "Hips"
    key_bone(arm, hips, [(0, {"location": (0, 0, 0)}), (30, {"location": (0, 0, 0)})])


def write_fbx(path):
    bpy.ops.export_scene.fbx(
        filepath=path,
        object_types={"ARMATURE", "MESH"},
        bake_anim=True,
        add_leaf_bones=False,
        bake_anim_use_nla_strips=False,
    )


def make_clip(path, still=False):
    reset()
    arm = armature("Armature", T_POSE, lambda n: PREFIX + n)
    skinned_box(arm)
    dodge(arm, PREFIX + "Hips", PREFIX + "LeftArm", still)
    write_fbx(path)


def make_slide(path):
    reset()
    arm = armature("Armature", T_POSE, lambda n: PREFIX + n)
    skinned_box(arm)
    sway(arm, PREFIX + "LeftUpLeg")
    write_fbx(path)


def make_custom(path):
    reset()
    arm = armature("Armature", CHAIN)
    arm.animation_data_create().action = bpy.data.actions.new("Sway")
    key_bone(arm, "seg_b", [(0, {"rotation_quaternion": (1, 0, 0, 0)}), (24, {"rotation_quaternion": (0.98, 0.2, 0, 0)})])
    bpy.context.scene.render.fps = 24
    write_fbx(path)


def make_model(path):
    reset()
    arm = armature("elsheria_rig", a_pose(T_POSE), lambda n: MODEL_NAMES[n])
    skinned_box(arm, size=0.4)
    arm.animation_data_create().action = bpy.data.actions.new("idle")
    key_bone(arm, "chest", [(0, {"rotation_quaternion": (1, 0, 0, 0)}), (30, {"rotation_quaternion": (0.995, 0.1, 0, 0)})])
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(filepath=path, export_format="GLB", export_animations=True, export_yup=True)


def main():
    argv = sys.argv[sys.argv.index("--") + 1 :]
    which, out = argv[0], os.path.abspath(argv[1])
    os.makedirs(os.path.dirname(out), exist_ok=True)
    if which == "clip":
        make_clip(out)
    elif which == "still":
        make_clip(out, still=True)
    elif which == "slide":
        make_slide(out)
    elif which == "custom":
        make_custom(out)
    elif which == "model":
        make_model(out)
    else:
        raise SystemExit(f"unknown fixture {which!r}")
    print(f"[fixture] wrote {out}", flush=True)


main()
