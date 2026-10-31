"""rig_models.py: small rigged test models for the rig doctor, built from nothing in headless Blender.

Usage: blender -b --factory-startup -P rig_models.py -- <which> out.glb
  humanoid    separate limbs, a cape on the arm bones, hair on the finger bones, a walk and a fall clip
  robed       the same, with a robe over the front: its side panels sit on the arm bones and split from the
              middle at a UV seam, so raising the arms cracks it open, and a cast clip that does that
  webbed      a clean rig, every vertex on its nearest bones, with cloth webs from each arm to the trunk:
              raising the arms stretches them as it would any cloth, and no weight is wrong
  misplaced   the same, with the left arm chain 10 cm inside the body and arm translation keys
  one-island  the same body remeshed into one continuous shell
  facing-x    humanoid turned to face +X in the file
  long-robe   a clean rig under a floor length robe whose weights blend from the hips to the legs, and a wide walk
  robe-torn   the same robe with rows of it on the hips and on the thigh in turn: a real tear in the walk
  robe-arm    the same robe with its hem blended smoothly onto the far arm bone: a real stretch in raise
  creature    a four legged beast facing -X: swapped side names, a junk chain, split spikes, a tail on a leg
  hound       a four legged beast as a Tripo rig names it: toes forked at each paw, all weighted, a nub on a
              leg, a long tail and weighted back spikes
  hound-shell the same remeshed into one shell with many UV islands and clean weights: the fix has nothing to do
  no-skin     bones and a mesh with no weights
"""
import bpy, bmesh, math, sys
from mathutils import Vector

which, out = sys.argv[sys.argv.index("--") + 1:][:2]
P = "mixamorig:"


def tube(bm, a, b, r0, r1=None, sides=10, rings=8):
    a, b = Vector(a), Vector(b)
    r1 = r0 if r1 is None else r1
    d = (b - a).normalized()
    u = d.orthogonal().normalized()
    v = d.cross(u)
    loops = []
    for i in range(rings + 1):
        t = i / rings
        c = a.lerp(b, t)
        r = r0 + (r1 - r0) * t
        loops.append([bm.verts.new(c + (u * math.cos(k * 2 * math.pi / sides) + v * math.sin(k * 2 * math.pi / sides)) * r) for k in range(sides)])
    for i in range(rings):
        for k in range(sides):
            k2 = (k + 1) % sides
            bm.faces.new((loops[i][k], loops[i][k2], loops[i + 1][k2], loops[i + 1][k]))
    bm.faces.new(loops[0][::-1])
    bm.faces.new(loops[-1])


def panel(bm, corners, nu=12, nv=20):
    a, b, c, d = [Vector(x) for x in corners]
    grid = [[bm.verts.new(a.lerp(b, i / nu).lerp(d.lerp(c, i / nu), j / nv)) for i in range(nu + 1)] for j in range(nv + 1)]
    for j in range(nv):
        for i in range(nu):
            bm.faces.new((grid[j][i], grid[j][i + 1], grid[j + 1][i + 1], grid[j + 1][i]))


def make_arm(joints):
    data = bpy.data.armatures.new("Armature")
    arm = bpy.data.objects.new("Armature", data)
    bpy.context.scene.collection.objects.link(arm)
    bpy.context.view_layer.objects.active = arm
    arm.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    made = {}
    for name, head, tail, parent in joints:
        b = data.edit_bones.new(name)
        b.head, b.tail = Vector(head), Vector(tail)
        if parent:
            b.parent = made[parent]
        made[name] = b
    bpy.ops.object.mode_set(mode="OBJECT")
    return arm


def seg_dist(p, a, b):
    ab = b - a
    t = max(0.0, min(1.0, (p - a).dot(ab) / max(ab.dot(ab), 1e-9)))
    return (p - (a + ab * t)).length


def finish(bm, arm, weigh, name="Body"):
    """A mesh with planar UVs, a small texture and the weights weigh(co) gives, skinned to arm."""
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    if which in ("one-island", "hound-shell"):
        ob0 = bpy.data.objects.new("tmp", me)
        bpy.context.scene.collection.objects.link(ob0)
        m = ob0.modifiers.new("r", "REMESH")
        m.mode, m.voxel_size = "VOXEL", 0.012
        dg = bpy.context.evaluated_depsgraph_get()
        me = bpy.data.meshes.new_from_object(ob0.evaluated_get(dg))
        bpy.data.objects.remove(ob0)
    ob = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(ob)
    uv = me.uv_layers.new(name="UVMap")
    for li, loop in enumerate(me.loops):
        co = me.vertices[loop.vertex_index].co
        uv.data[li].uv = (co.x + 0.5, co.z)
    if which == "hound-shell":   # a patchwork of UV islands, so its vertices split along them in the file
        for poly in me.polygons:
            c = poly.center
            for li in poly.loop_indices:
                uv.data[li].uv = (uv.data[li].uv[0] + int((c.x + 2) * 6) * 0.5, uv.data[li].uv[1] + int(c.z * 6) * 0.5)
    img = bpy.data.images.new("tex", 16, 16)
    img.pixels = [c for i in range(256) for c in (0.5 + (i % 16) / 40, 0.45, 0.4 + (i // 16) / 40, 1)]
    img.pack()
    mat = bpy.data.materials.new("skin")
    mat.use_nodes = True
    node = mat.node_tree.nodes.new("ShaderNodeTexImage")
    node.image = img
    mat.node_tree.links.new(node.outputs["Color"], mat.node_tree.nodes["Principled BSDF"].inputs["Base Color"])
    me.materials.append(mat)
    ob.parent = arm
    ob.modifiers.new("Armature", "ARMATURE").object = arm
    if weigh is None:
        return ob
    groups = {}
    for v in me.vertices:
        for bone, w in weigh(v.co.copy()).items():
            if bone not in groups:
                groups[bone] = ob.vertex_groups.new(name=bone)
            groups[bone].add([v.index], w, "REPLACE")
    return ob


def humanoid(shift_left_arm=0.0):
    """A character one unit tall facing -Y, left on +X, arms hanging out in an A pose."""
    s = shift_left_arm
    def side(name, x):
        q = -s if x > 0 else 0.0   # a misplaced left chain sits nearer the body
        k = 1 if x > 0 else -1
        J = [(f"{P}{name}Shoulder", (k * 0.03, 0, 0.80), (k * 0.09, 0, 0.80), P + "Spine2"),
             (f"{P}{name}Arm", (k * 0.09 + k * q, 0, 0.80), (k * 0.20 + k * q, 0, 0.66), f"{P}{name}Shoulder"),
             (f"{P}{name}ForeArm", (k * 0.20 + k * q, 0, 0.66), (k * 0.30 + k * q, 0, 0.53), f"{P}{name}Arm"),
             (f"{P}{name}Hand", (k * 0.30 + k * q, 0, 0.53), (k * 0.32 + k * q, 0, 0.50), f"{P}{name}ForeArm")]
        for f, dy in (("Thumb", -0.02), ("Index", -0.007), ("Middle", 0.007), ("Pinky", 0.02)):
            par = f"{P}{name}Hand"
            for n in range(1, 5):
                h = (k * (0.32 + (n - 1) * 0.012) + k * q, dy, 0.50 - (n - 1) * 0.015)
                t = (k * (0.32 + n * 0.012) + k * q, dy, 0.50 - n * 0.015)
                J.append((f"{P}{name}Hand{f}{n}", h, t, par))
                par = f"{P}{name}Hand{f}{n}"
        l = f"{P}{name}"
        J += [(l + "UpLeg", (k * 0.07, 0, 0.50), (k * 0.08, 0, 0.27), P + "Hips"),
              (l + "Leg", (k * 0.08, 0, 0.27), (k * 0.08, 0, 0.05), l + "UpLeg"),
              (l + "Foot", (k * 0.08, 0, 0.05), (k * 0.08, -0.06, 0.015), l + "Leg"),
              (l + "ToeBase", (k * 0.08, -0.06, 0.015), (k * 0.08, -0.11, 0.015), l + "Foot"),
              (l + "Toe_End", (k * 0.08, -0.11, 0.015), (k * 0.08, -0.13, 0.015), l + "ToeBase")]
        return J
    joints = [(P + "Hips", (0, 0, 0.52), (0, 0, 0.58), None), (P + "Spine", (0, 0, 0.58), (0, 0, 0.65), P + "Hips"),
              (P + "Spine1", (0, 0, 0.65), (0, 0, 0.72), P + "Spine"), (P + "Spine2", (0, 0, 0.72), (0, 0, 0.82), P + "Spine1"),
              (P + "Neck", (0, 0, 0.82), (0, 0, 0.87), P + "Spine2"), (P + "Head", (0, 0, 0.87), (0, 0, 0.99), P + "Neck"),
              (P + "HeadTop_End", (0, 0, 0.99), (0, 0, 1.02), P + "Head")]
    for name, x in (("Left", 1), ("Right", -1)):
        joints += side(name, x)
    arm = make_arm(joints)
    segs = {n: (Vector(h), Vector(t)) for n, h, t, _ in joints}
    bm = bmesh.new()
    tube(bm, (0, 0, 0.50), (0, 0, 0.82), 0.10, 0.085, 14, 12)
    tube(bm, (0, 0, 0.82), (0, 0, 0.87), 0.035, 0.035)
    tube(bm, (0, 0, 0.87), (0, 0, 1.00), 0.06, 0.05, 12, 8)
    for k in (1, -1):
        tube(bm, (k * 0.09, 0, 0.80), (k * 0.20, 0, 0.66), 0.03, 0.026)
        tube(bm, (k * 0.20, 0, 0.66), (k * 0.30, 0, 0.53), 0.026, 0.02)
        tube(bm, (k * 0.30, 0, 0.53), (k * 0.32, 0, 0.50), 0.02, 0.018, 8, 4)
        for dy in (-0.02, -0.007, 0.007, 0.02):
            tube(bm, (k * 0.32, dy, 0.50), (k * 0.368, dy, 0.44), 0.006, 0.005, 6, 8)
        tube(bm, (k * 0.07, 0, 0.50), (k * 0.08, 0, 0.27), 0.05, 0.035)
        tube(bm, (k * 0.08, 0, 0.27), (k * 0.08, 0, 0.05), 0.033, 0.03)
        tube(bm, (k * 0.08, 0, 0.05), (k * 0.08, -0.13, 0.015), 0.03, 0.02, 8, 6)
    robe = which in ("long-robe", "robe-torn", "robe-arm")
    if robe:
        tube(bm, (0, 0, 0.56), (0, 0, 0.08), 0.17, 0.24, 28, 24)
    if which != "webbed" and not robe:
        panel(bm, ((-0.12, 0.095, 0.80), (0.12, 0.095, 0.80), (0.18, 0.13, 0.15), (-0.18, 0.13, 0.15)))   # the cape
        for x in (-0.03, 0.03):
            tube(bm, (x, 0.07, 0.97), (x * 1.5, 0.10, 0.74), 0.012, 0.006, 6, 14)                         # hair strands
    if which == "robed":   # the copies along each seam differ by less than a seam reads as apart
        for k in (1, -1):
            panel(bm, ((k * (0.12 + 1e-6), -0.11, 0.80), (k * 0.30, -0.11, 0.70), (k * 0.30, -0.11, 0.25), (k * (0.12 + 1e-6), -0.11, 0.25)), 4, 8)
        panel(bm, ((-0.12, -0.11, 0.80), (0.12, -0.11, 0.80), (0.12, -0.11, 0.25), (-0.12, -0.11, 0.25)), 6, 8)
    if which == "webbed":
        for k in (1, -1):
            panel(bm, ((k * 0.17, 0.0, 0.70), (k * 0.09, 0.0, 0.62), (k * 0.09, 0.0, 0.56), (k * 0.25, 0.0, 0.60)), 1, 1)
    hand_bones = {k: [n for n in segs if n.startswith(f"{P}{k}Hand")] for k in ("Left", "Right")}
    arm_bones = {k: [f"{P}{k}Arm", f"{P}{k}ForeArm"] for k in ("Left", "Right")}
    all_names = list(segs)
    def weigh(co):
        ds = sorted(((seg_dist(co, *segs[n]), n) for n in all_names if n != P + "HeadTop_End"), key=lambda x: x[0])[:3]
        iw = {n: 1 / max(d, 0.005) ** 4 for d, n in ds}
        total = sum(iw.values())
        out_ = {n: w / total for n, w in iw.items()}
        kx = "Left" if co.x > 0 else "Right"
        if which == "robed" and abs(co.y + 0.11) < 1e-4:
            if abs(co.x) > 0.12 + 5e-7:
                return {P + kx + "Arm": 0.7, P + "Spine2": 0.3}
            return {P + "Spine2": 1.0}
        if which == "webbed":
            return out_
        if robe:
            r = (co.x ** 2 + co.y ** 2) ** 0.5
            if r < 0.16 or co.z > 0.565 or co.z < 0.075:   # the body under it
                return out_
            down = min(max((0.56 - co.z) / 0.48, 0.0), 1.0)
            left = min(max((co.x + 0.17) / 0.34, 0.0), 1.0)
            left = left * left * (3 - 2 * left)
            kx_leg = lambda s: {P + s + "UpLeg": 0.6, P + s + "Leg": 0.4}
            blend = {P + "Hips": 1 - down}
            for s, share in (("Left", left), ("Right", 1 - left)):
                for b, w in kx_leg(s).items():
                    blend[b] = down * share * w
            if which == "robe-torn" and co.y < 0 and co.x > 0 and 0.16 < co.z < 0.40:
                return {P + "LeftUpLeg": 1.0} if int(co.z / 0.02) % 2 else {P + "Hips": 1.0}
            if which == "robe-arm" and co.x > 0 and co.z < 0.30:
                onto = min((0.30 - co.z) / 0.20, 1.0)
                blend = {b: w * (1 - onto) for b, w in blend.items()}
                blend[P + "LeftArm"] = onto
            return {b: w for b, w in blend.items() if w > 0}
        if co.y > 0.09 and co.z < 0.81 and abs(co.x) < 0.2:          # the cape swings with the arms
            return {P + kx + "Arm": 0.55, P + kx + "ForeArm": 0.15, P + "Spine2": 0.3}
        if co.y > 0.055 and co.z > 0.72 and abs(co.x) < 0.06:         # hair on the fingers
            return {P + kx + "HandIndex1": 0.6, P + "Head": 0.4}
        return out_
    finish(bm, arm, weigh)
    arm.animation_data_create()
    act = bpy.data.actions.new("walk")
    arm.animation_data.action = act
    stride = 50 if robe else 20
    for name, deg in ((P + "LeftUpLeg", stride), (P + "RightUpLeg", -stride)):
        pb = arm.pose.bones[name]
        pb.rotation_mode = "XYZ"
        for f, sign in ((1, 1), (15, -1), (30, 1)):
            pb.rotation_euler = (math.radians(deg * sign), 0, 0)
            pb.keyframe_insert("rotation_euler", frame=f)
    # A fall: the body tips over and sinks under where it stood, as a death clip does, for the framing test.
    fall = bpy.data.actions.new("fall")
    arm.animation_data.action = fall
    hips = arm.pose.bones[P + "Hips"]
    hips.rotation_mode = "XYZ"
    for f, deg, down in ((1, 0, 0.0), (30, 88, 0.72)):
        hips.rotation_euler = (math.radians(deg), 0, 0)
        hips.location = (0, down, 0)
        hips.keyframe_insert("rotation_euler", frame=f)
        hips.keyframe_insert("location", frame=f)
    arm.animation_data.nla_tracks.new().strips.new("fall", 1, fall)
    if which in ("robed", "webbed"):   # both arms up, as a cast does
        cast = bpy.data.actions.new("cast")
        arm.animation_data.action = cast
        for name, sign in ((P + "LeftArm", 1), (P + "RightArm", -1)):
            pb = arm.pose.bones[name]
            pb.rotation_mode = "XYZ"
            for f, deg in ((1, 0), (30, 90)):
                pb.rotation_euler = (0, 0, math.radians(-deg * sign))
                pb.keyframe_insert("rotation_euler", frame=f)
            pb.rotation_euler = (0, 0, 0)
        arm.animation_data.nla_tracks.new().strips.new("cast", 1, cast)
    arm.animation_data.action = act
    hips.rotation_euler = (0, 0, 0)
    hips.location = (0, 0, 0)
    if s:
        pb = arm.pose.bones[P + "LeftArm"]
        for f, y in ((1, 0.0), (30, 0.002)):
            pb.location = (0, y, 0)
            pb.keyframe_insert("location", frame=f)
    return arm


def creature():
    """A beast about one unit long facing -X. Rear side names are swapped."""
    R = "tripo::Root"
    joints = [(R, (0.30, 0, 0.45), (0.15, 0, 0.45), None), ("bone_1", (0.15, 0, 0.45), (0.0, 0, 0.46), R), ("bone_2", (0.0, 0, 0.46), (-0.20, 0, 0.47), "bone_1"),
              ("bone_3", (-0.20, 0, 0.47), (-0.32, 0, 0.56), "bone_2"), ("bone_4", (-0.32, 0, 0.56), (-0.44, 0, 0.60), "bone_3"),
              ("tail_1", (0.30, 0, 0.45), (0.42, 0, 0.48), R), ("tail_2", (0.42, 0, 0.48), (0.55, 0, 0.52), "tail_1"), ("tail_3", (0.55, 0, 0.52), (0.68, 0, 0.57), "tail_2")]
    # (name prefix, x, y): left is -Y for a beast that faces -X. The rear pair carries swapped names.
    legs = (("FL", -0.18, -0.10, "bone_2"), ("FR", -0.18, 0.10, "bone_2"), ("Right_rear", 0.25, -0.10, R), ("Left_rear", 0.25, 0.10, R))
    for nm, x, y, par in legs:
        p = par
        for i, (z0, z1) in enumerate(((0.42, 0.28), (0.28, 0.14), (0.14, 0.0)), 1):
            joints.append((f"{nm}_{i}", (x, y, z0), (x, y, z1), p))
            p = f"{nm}_{i}"
    joints += [("dup_FL", (-0.18, -0.10, 0.415), (-0.18, -0.10, 0.285), "bone_2"), ("dup_FR", (-0.18, 0.10, 0.415), (-0.18, 0.10, 0.285), "bone_2")]
    spikes = [(0.05 + 0.1 * i, 0.47) for i in range(4)]
    for i, (x, z) in enumerate(spikes):
        joints.append((f"spike_{i}", (x, 0, z + 0.05), (x, 0, z + 0.14), "bone_1"))
    arm = make_arm(joints)
    segs = {n: (Vector(h), Vector(t)) for n, h, t, _ in joints}
    bm = bmesh.new()
    tube(bm, (0.32, 0, 0.45), (-0.2, 0, 0.47), 0.11, 0.10, 14, 24)
    tube(bm, (-0.2, 0, 0.47), (-0.32, 0, 0.56), 0.08, 0.06, 12, 10)
    tube(bm, (-0.32, 0, 0.56), (-0.46, 0, 0.60), 0.07, 0.04, 12, 10)
    tube(bm, (0.30, 0, 0.45), (0.68, 0, 0.57), 0.03, 0.008, 8, 24)
    for nm, x, y, _ in legs:
        tube(bm, (x, y, 0.42), (x, y, 0.0), 0.035, 0.022, 10, 20)
    for x, z in spikes:
        tube(bm, (x, 0, z + 0.08), (x, 0, z + 0.16), 0.02, 0.002, 6, 4)
    def weigh(co):
        names = [n for n in segs if not n.startswith(("dup", "spike"))]
        if co.z > 0.54 and -0.1 < co.x < 0.4:                         # a spike split between two bones
            i = min(range(len(spikes)), key=lambda k: abs(co.x - spikes[k][0]))
            return {f"spike_{i}": 0.5, "bone_1": 0.5}
        ds = sorted(((seg_dist(co, *segs[n]), n) for n in names), key=lambda x: x[0])[:3]
        iw = {n: 1 / max(d, 0.005) ** 4 for d, n in ds}
        total = sum(iw.values())
        w = {n: v / total for n, v in iw.items()}
        if co.x > 0.5:                                                # the tail tip pulls the front right leg
            w = {k: v * 0.7 for k, v in w.items()}
            w["FR_1"] = 0.3
        if abs(co.x + 0.18) < 0.05 and co.z < 0.40 and co.z > 0.25:   # the junk chain carries weight
            side = "dup_FL" if co.y < 0 else "dup_FR"
            w = {k: v * 0.9 for k, v in w.items()}
            w[side] = 0.1
        return w
    finish(bm, arm, weigh)
    return arm


def hound():
    """A beast about one unit long facing -X, with the bone tree a Tripo rig gives a quadruped."""
    R = "tripo::Root"
    joints = [(R, (0.30, 0, 0.50), (0.15, 0, 0.50), None), ("bone_1", (0.15, 0, 0.50), (0.0, 0, 0.51), R), ("bone_2", (0.0, 0, 0.51), (-0.20, 0, 0.52), "bone_1"),
              ("bone_3", (-0.20, 0, 0.52), (-0.32, 0, 0.60), "bone_2"), ("bone_4", (-0.32, 0, 0.60), (-0.46, 0, 0.65), "bone_3")]
    joints.append(("bone_40", (0.30, 0, 0.50), (0.38, 0, 0.53), R))
    for i in range(1, 6):
        joints.append((f"bone_{40 + i}", (0.30 + 0.08 * i, 0, 0.50 + 0.03 * i), (0.38 + 0.08 * i, 0, 0.53 + 0.03 * i), f"bone_{40 + i - 1}"))
    legs = ((0, -0.18, -0.11, "bone_2"), (1, -0.18, 0.11, "bone_2"), (2, 0.25, -0.11, R), (3, 0.25, 0.11, R))
    paws = []
    for n, x, y, par in legs:
        p = par
        for i, (z0, z1) in enumerate(((0.46, 0.30), (0.30, 0.15), (0.15, 0.05)), 1):
            joints.append((f"tripo::{n}_Limb_{i}", (x, y, z0), (x, y, z1), p))
            p = f"tripo::{n}_Limb_{i}"
        for t, dy in enumerate((-0.03, 0.0, 0.03), 1):   # toes fork from the paw, one long enough to read as a chain by size
            tip = -0.15 if t == 2 else -0.07
            joints.append((f"bone_{n}{t}", (x, y, 0.05), (x + tip, y + dy, 0.01), p))
            paws.append((x, y, dy, tip))
    joints.append(("bone_99", (-0.18, -0.11, 0.22), (-0.13, -0.11, 0.20), "tripo::0_Limb_2"))   # a nub by a front leg
    spikes = [(0.05 + 0.1 * i, 0.60) for i in range(3)]
    for i, (x, z) in enumerate(spikes):
        joints.append((f"bone_6{i}", (x, 0, z), (x, 0, z + 0.09), "bone_1"))
    arm = make_arm(joints)
    segs = {n: (Vector(h), Vector(t)) for n, h, t, _ in joints}
    bm = bmesh.new()
    tube(bm, (0.32, 0, 0.50), (-0.2, 0, 0.52), 0.11, 0.10, 14, 24)
    tube(bm, (-0.2, 0, 0.52), (-0.32, 0, 0.60), 0.08, 0.06, 12, 10)
    tube(bm, (-0.32, 0, 0.60), (-0.48, 0, 0.65), 0.07, 0.04, 12, 10)
    tube(bm, (0.30, 0, 0.50), (0.78, 0, 0.68), 0.03, 0.008, 8, 24)
    for n, x, y, _ in legs:
        tube(bm, (x, y, 0.46), (x, y, 0.05), 0.04, 0.025, 10, 20)
    for x, y, dy, tip in paws:
        tube(bm, (x, y, 0.05), (x + tip, y + dy, 0.01), 0.018, 0.01, 6, 6)
    tube(bm, (-0.18, -0.11, 0.22), (-0.13, -0.11, 0.20), 0.016, 0.008, 6, 3)
    for x, z in spikes:
        tube(bm, (x, 0, z), (x, 0, z + 0.09), 0.02, 0.003, 6, 4)
    def weigh(co):
        ds = sorted((seg_dist(co, *s), n) for n, s in segs.items())[:2]
        iw = {n: 1 / max(d, 0.005) ** 6 for d, n in ds}
        total = sum(iw.values())
        return {n: v / total for n, v in iw.items()}
    finish(bm, arm, weigh)
    return arm


bpy.ops.wm.read_factory_settings(use_empty=True)
if which == "creature":
    arm = creature()
elif which in ("hound", "hound-shell"):
    arm = hound()
elif which == "misplaced":
    arm = humanoid(0.05)
else:
    arm = humanoid()
if which == "no-skin":
    for g in list(bpy.data.objects["Body"].vertex_groups):
        bpy.data.objects["Body"].vertex_groups.remove(g)
    bpy.data.objects["Body"].modifiers.clear()
if which == "facing-x":
    bpy.context.view_layer.objects.active = arm
    arm.rotation_euler = (0, 0, math.radians(90))     # the face (-Y) turns to +X
    bpy.context.view_layer.update()
    bpy.ops.object.select_all(action="DESELECT")
    arm.select_set(True)
    bpy.data.objects["Body"].select_set(True)
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)
bpy.ops.object.select_all(action="SELECT")
bpy.ops.export_scene.gltf(filepath=out, export_format="GLB", export_yup=True, export_animations=True, export_skins=True, export_image_format="AUTO", use_selection=True)
print("MODEL", out)
