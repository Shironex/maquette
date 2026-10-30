"""Imports a .vrm with the VRM Add-on for Blender, headless, and prints what
the add-on read: the spec version, the required humanoid bones it found,
the meta, and the left arm's direction (upper arm to hand, at rest).

A second opinion on a file Maquette wrote, from another VRM reader. Not
part of the app or of CI; the add-on is not vendored. Install it first
(see README.md), then run:

    blender -b --factory-startup --python-exit-code 1 \\
        --python vrm_addon_check.py -- /abs/path/avatar.vrm

It prints one JSON object and exits 1 when the import fails or a required
bone is missing.
"""

import json
import sys

import addon_utils
import bpy

# The 15 bones VRM 1.0 requires.
REQUIRED = [
    "hips",
    "spine",
    "head",
    "leftUpperLeg",
    "leftLowerLeg",
    "leftFoot",
    "rightUpperLeg",
    "rightLowerLeg",
    "rightFoot",
    "leftUpperArm",
    "leftLowerArm",
    "leftHand",
    "rightUpperArm",
    "rightLowerArm",
    "rightHand",
]


def arguments():
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    if len(args) != 1:
        raise SystemExit("usage: blender -b --python vrm_addon_check.py -- /abs/path/avatar.vrm")
    return args[0]


def enable_addon():
    """Enables the add-on, installed as an extension (bl_ext.<repo>.vrm) or a legacy add-on."""
    names = [m.__name__ for m in addon_utils.modules()]
    found = [n for n in names if n.endswith(".vrm") or n in ("io_scene_vrm", "VRM_Addon_for_Blender-release")]
    if not found:
        raise SystemExit("The VRM Add-on for Blender is not installed. See README.md.")
    addon_utils.enable(found[0], default_set=True)
    return found[0]


def clear_scene():
    for obj in list(bpy.data.objects):
        bpy.data.objects.remove(obj, do_unlink=True)


def armature_with_vrm():
    for obj in bpy.data.objects:
        if obj.type == "ARMATURE" and hasattr(obj.data, "vrm_addon_extension"):
            return obj
    return None


def bone_map(ext):
    """Each VRM bone name to the Blender bone the add-on mapped it to."""
    out = {}
    for name, human in ext.vrm1.humanoid.human_bones.human_bone_name_to_human_bone().items():
        bone = human.node.bone_name
        if bone:
            out[getattr(name, "value", str(name))] = bone
    return out


def meta_of(ext):
    meta = ext.vrm1.meta
    return {
        "name": meta.vrm_name,
        "version": meta.version,
        "authors": [a.value for a in meta.authors],
        "avatarPermission": meta.avatar_permission,
        "commercialUsage": meta.commercial_usage,
        "creditNotation": meta.credit_notation,
        "allowRedistribution": meta.allow_redistribution,
        "modification": meta.modification,
    }


def left_arm(armature, bones):
    """The unit vector from the left upper arm's head to the left hand's head, in world space."""
    upper, hand = bones.get("leftUpperArm"), bones.get("leftHand")
    if not upper or not hand:
        return None
    world = armature.matrix_world
    start = world @ armature.data.bones[upper].head_local
    end = world @ armature.data.bones[hand].head_local
    v = (end - start).normalized()
    # Blender is Z up; glTF and VRM are Y up with +Z forward: (x, z, -y).
    return {
        "blender": [round(c, 4) for c in v],
        "gltf": [round(v.x, 4), round(v.z, 4), round(-v.y, 4)],
    }


def main():
    path = arguments()
    module = enable_addon()
    clear_scene()
    result = bpy.ops.import_scene.vrm(filepath=path)
    if "FINISHED" not in result:
        raise SystemExit(f"The add-on did not import {path}: {result}")
    armature = armature_with_vrm()
    if armature is None:
        raise SystemExit("The import made no armature with VRM data.")
    ext = armature.data.vrm_addon_extension
    bones = bone_map(ext)
    missing = [b for b in REQUIRED if b not in bones]
    print(
        json.dumps(
            {
                "addon": module,
                "spec": ext.spec_version,
                "bones": len(bones),
                "required": len(REQUIRED) - len(missing),
                "missing": missing,
                "meta": meta_of(ext),
                "leftArm": left_arm(armature, bones),
            },
            indent=2,
        )
    )
    if missing:
        sys.exit(1)


main()
