"""Small models that move, for the Motion tests and the mock UI. Run headless:

    blender -b --factory-startup --python motion_models.py -- <which> <out.glb>

<which> is one of:
- figure: a clay figure 1.2 m tall on a 20 bone humanoid rig (snake case
  names, no toe bones) with three clips: "Walk loop" (32 frames at 30 fps,
  walking 1.4 m forward), "Idle breathe" (90 frames) and "Wave" (48 frames).
  Every box is weighted to one bone.
- rest: the same figure and rig with no clips, as a model fresh from the
  rigging step.
- cabinet: a switch cabinet in parts and no clips: its body, a door hinged
  at its left edge and six breakers, the parts a state moves.
- turntable: a round plate with one object clip, "Spin" (120 frames).
"""

import math
import sys

import bpy
from mathutils import Matrix, Vector

FPS = 30


def reset():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.render.fps = FPS
    return scene


def material(name, rgb):
    mat = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    if mat.node_tree is None:
        mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    if bsdf is not None:
        bsdf.inputs["Base Color"].default_value = (*rgb, 1.0)
        bsdf.inputs["Roughness"].default_value = 0.7
    return mat


def box_data(name, size, centre=(0.0, 0.0, 0.0)):
    """A box mesh, `size` x y z in metres, around `centre`."""
    sx, sy, sz = (s / 2 for s in size)
    cx, cy, cz = centre
    verts = [
        (cx + x * sx, cy + y * sy, cz + z * sz)
        for x in (-1, 1)
        for y in (-1, 1)
        for z in (-1, 1)
    ]
    faces = [
        (0, 1, 3, 2),
        (4, 6, 7, 5),
        (0, 4, 5, 1),
        (2, 3, 7, 6),
        (0, 2, 6, 4),
        (1, 5, 7, 3),
    ]
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    return mesh


def link(obj, scene):
    scene.collection.objects.link(obj)
    return obj


def export(path, use_animation=True):
    bpy.ops.export_scene.gltf(
        filepath=path,
        export_format="GLB",
        export_animations=use_animation,
        export_animation_mode="ACTIONS",
        export_yup=True,
        export_cameras=False,
        export_lights=False,
    )


# The figure: bone name, parent, head, tail (Blender axes, Z up, -Y forward).
BONES = [
    ("hips", None, (0, 0, 0.62), (0, 0, 0.70)),
    ("spine_01", "hips", (0, 0, 0.70), (0, 0, 0.78)),
    ("spine_02", "spine_01", (0, 0, 0.78), (0, 0, 0.86)),
    ("spine_03", "spine_02", (0, 0, 0.86), (0, 0, 0.94)),
    ("neck", "spine_03", (0, 0, 0.94), (0, 0, 1.0)),
    ("head", "neck", (0, 0, 1.0), (0, 0, 1.2)),
]
for side, s in (("l", 1), ("r", -1)):
    BONES += [
        (f"clavicle_{side}", "spine_03", (s * 0.03, 0, 0.92), (s * 0.12, 0, 0.92)),
        (f"upperarm_{side}", f"clavicle_{side}", (s * 0.12, 0, 0.92), (s * 0.30, 0, 0.92)),
        (f"lowerarm_{side}", f"upperarm_{side}", (s * 0.30, 0, 0.92), (s * 0.46, 0, 0.92)),
        (f"hand_{side}", f"lowerarm_{side}", (s * 0.46, 0, 0.92), (s * 0.54, 0, 0.92)),
        (f"thigh_{side}", "hips", (s * 0.07, 0, 0.62), (s * 0.07, 0, 0.34)),
        (f"calf_{side}", f"thigh_{side}", (s * 0.07, 0, 0.34), (s * 0.07, 0, 0.06)),
        (f"foot_{side}", f"calf_{side}", (s * 0.07, 0, 0.06), (s * 0.07, -0.1, 0.02)),
    ]

# A box per bone: size, as a share along the bone for its centre.
SHAPES = {
    "hips": (0.22, 0.14, 0.12),
    "spine_01": (0.2, 0.13, 0.1),
    "spine_02": (0.22, 0.14, 0.1),
    "spine_03": (0.26, 0.15, 0.1),
    "neck": (0.07, 0.07, 0.07),
    "head": (0.16, 0.17, 0.2),
    "clavicle": (0.1, 0.08, 0.07),
    "upperarm": (0.18, 0.07, 0.07),
    "lowerarm": (0.16, 0.06, 0.06),
    "hand": (0.08, 0.05, 0.03),
    "thigh": (0.09, 0.09, 0.28),
    "calf": (0.08, 0.08, 0.28),
    "foot": (0.08, 0.14, 0.05),
}


def figure(scene, out, clips=True):
    arm_data = bpy.data.armatures.new("Armature")
    rig = link(bpy.data.objects.new("Armature", arm_data), scene)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="EDIT")
    edit = {}
    for name, parent, head, tail in BONES:
        bone = arm_data.edit_bones.new(name)
        bone.head, bone.tail = head, tail
        if parent:
            bone.parent = edit[parent]
            bone.use_connect = False
        edit[name] = bone
    bpy.ops.object.mode_set(mode="OBJECT")

    verts, faces, groups = [], [], []
    for name, _, head, tail in BONES:
        key = name if name in SHAPES else name.rsplit("_", 1)[0]
        size = SHAPES[key]
        centre = (Vector(head) + Vector(tail)) / 2
        if name == "foot_l" or name == "foot_r":
            centre = Vector((head[0], -0.03, 0.03))
        # Arms reach along X, the rest along their own length.
        mesh = box_data("tmp", size, centre)
        base = len(verts)
        verts += [v.co.copy() for v in mesh.vertices]
        faces += [tuple(base + i for i in p.vertices) for p in mesh.polygons]
        groups.append((name, range(base, base + len(mesh.vertices))))
        bpy.data.meshes.remove(mesh)
    body_mesh = bpy.data.meshes.new("Clay figure")
    body_mesh.from_pydata(verts, [], faces)
    body_mesh.update()
    body_mesh.materials.append(material("clay", (0.62, 0.33, 0.22)))
    body = link(bpy.data.objects.new("Clay figure", body_mesh), scene)
    for name, indices in groups:
        group = body.vertex_groups.new(name=name)
        group.add(list(indices), 1.0, "REPLACE")
    body.parent = rig
    modifier = body.modifiers.new("Armature", "ARMATURE")
    modifier.object = rig

    rig.animation_data_create()
    pose = rig.pose.bones

    def key(frame, values):
        """values: bone name to (euler degrees xyz) or ("loc", vector)."""
        for name, value in values.items():
            bone = pose[name]
            if value[0] == "loc":
                bone.location = value[1]
                bone.keyframe_insert("location", frame=frame)
            else:
                bone.rotation_mode = "XYZ"
                bone.rotation_euler = [math.radians(a) for a in value]
                bone.keyframe_insert("rotation_euler", frame=frame)

    def clip(name, frames):
        action = bpy.data.actions.new(name)
        action.use_fake_user = True
        rig.animation_data.action = action
        for bone in pose:
            bone.location = (0, 0, 0)
            bone.rotation_mode = "XYZ"
            bone.rotation_euler = (0, 0, 0)
        frames()
        track = rig.animation_data.nla_tracks.new()
        track.name = name
        start = int(action.frame_range[0])
        track.strips.new(name, start, action)
        rig.animation_data.action = None

    def walk():
        steps = 32
        for f in range(0, steps + 1, 4):
            phase = math.sin(2 * math.pi * f / steps)
            lift = abs(math.cos(2 * math.pi * f / steps))
            # Up bones: local Y is up, local Z is forward (-Y in the scene).
            # Down bones (legs): local X is across, so +X tips them back.
            key(f, {
                "hips": ("loc", Vector((0, 0.015 * lift, 1.42 * f / steps))),
                "thigh_l": (-28 * phase, 0, 0),
                "thigh_r": (28 * phase, 0, 0),
                "calf_l": (30 * max(0.0, phase), 0, 0),
                "calf_r": (30 * max(0.0, -phase), 0, 0),
                "upperarm_l": (-72, 22 * phase, 0),
                "upperarm_r": (-72, 22 * phase, 0),
                "spine_02": (0, 6 * phase, 0),
            })

    def idle():
        for f in range(0, 91, 15):
            breath = math.sin(2 * math.pi * f / 90)
            key(f, {
                "spine_01": (2 * breath, 0, 0),
                "spine_03": (-2 * breath, 0, 0),
                "head": (3 * breath, 0, 0),
                "upperarm_l": (-74 + 2 * breath, 0, 0),
                "upperarm_r": (-74 + 2 * breath, 0, 0),
            })

    def wave():
        for f in range(0, 49, 6):
            swing = math.sin(2 * math.pi * f / 24)
            up = min(f, 48 - f, 12) / 12
            key(f, {
                "upperarm_r": (-72 + 135 * up, 0, 0),
                "lowerarm_r": (30 * up + 25 * swing * up, 0, 0),
                "upperarm_l": (-72, 0, 0),
                "head": (0, 0, 8 * up),
            })

    if clips:
        clip("Walk loop", walk)
        clip("Idle breathe", idle)
        clip("Wave", wave)
    bpy.ops.object.select_all(action="DESELECT")
    export(out, use_animation=clips)


def cabinet(scene, out):
    metal = material("cabinet metal", (0.22, 0.23, 0.25))
    cream = material("breaker", (0.86, 0.84, 0.78))
    body = link(bpy.data.objects.new("Body", box_data("Body", (0.6, 0.4, 1.2), (0, 0, 0.6))), scene)
    body.data.materials.append(metal)
    # The door's origin sits on its hinge, so a state turns it about Z there.
    door = link(
        bpy.data.objects.new("Door", box_data("Door", (0.58, 0.03, 1.16), (0.29, 0, 0))),
        scene,
    )
    door.data.materials.append(metal)
    door.location = (-0.29, -0.215, 0.6)
    for i in range(6):
        row, col = divmod(i, 3)
        part = bpy.data.objects.new(
            f"Breaker {i + 1}",
            box_data(f"Breaker {i + 1}", (0.08, 0.06, 0.16)),
        )
        part.data.materials.append(cream)
        part.location = (-0.15 + col * 0.15, -0.17, 0.9 - row * 0.3)
        link(part, scene)
    export(out, use_animation=False)


def turntable(scene, out):
    plate = bpy.data.meshes.new("Plate")
    n = 24
    verts = [(0.2 * math.cos(2 * math.pi * i / n), 0.2 * math.sin(2 * math.pi * i / n), z)
             for z in (0.0, 0.03) for i in range(n)]
    faces = [tuple(range(n))[::-1], tuple(range(n, 2 * n))]
    faces += [(i, (i + 1) % n, n + (i + 1) % n, n + i) for i in range(n)]
    plate.from_pydata(verts, [], faces)
    plate.update()
    plate.materials.append(material("plate", (0.75, 0.73, 0.7)))
    obj = link(bpy.data.objects.new("Turntable", plate), scene)
    obj.animation_data_create()
    action = bpy.data.actions.new("Spin")
    obj.animation_data.action = action
    for f, turn in ((0, 0.0), (120, 360.0)):
        obj.rotation_euler = (0, 0, math.radians(turn))
        obj.keyframe_insert("rotation_euler", frame=f)
    export(out)


def main():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    which, out = argv[0], argv[1]
    scene = reset()
    makers = {
        "figure": figure,
        "rest": lambda scene, out: figure(scene, out, clips=False),
        "cabinet": cabinet,
        "turntable": turntable,
    }
    makers[which](scene, out)
    print(f"[motion_models] wrote {which} to {out}", flush=True)


main()
