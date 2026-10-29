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
     "elbows": "keep"}

What it does, in this order:

1. Turn the armature and the meshes about the up axis so the face points
   to glTF +Z and the character's left lands on +X. The left comes from the
   left minus right upper legs and upper arms, as the readiness reads it.
2. Move them so the hips sit on the Y axis and the soles on Y = 0, and
   bake the turn and the move into the armature and the meshes.
3. Lift each arm at the shoulder (the upper arm's head) until upper arm to
   hand is level along X. The clavicles stay where they are and the hand
   and fingers ride along unchanged. With "elbows": "straighten" the lower
   arm is levelled too, so upper arm, lower arm and hand all lie on X.
4. Make that pose the rest: the armature modifier is applied on each
   skinned mesh, the pose becomes the armature's rest, and the modifier is
   added back. The vertex groups stay as they were; at most 4 influences
   per vertex and weights summing to 1 are checked and kept.
5. Export a plain GLB without animations.

The report holds the measures before and after (facing, each arm's drop,
the elbow bends, the soles, the hips, the height) in glTF metres and
degrees, the turn and the move, and the weight checks. The two pose sheets
show the model before (only turned to face the camera) and after, in rows
rest, arms down, arms raised and a forward step, and columns front, side,
back and a close look at the shoulders, so the armpits and whatever hangs
beside the arms (a cape, hair) can be judged. Poses are world space turns
about each bone's head, so they read the same on any rig.
"""

import argparse
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
CELL = 400
# Degrees the arms point below and above level in the down and raise poses.
DOWN_DEG = 65.0
RAISE_DEG = 45.0


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
    job.setdefault("elbows", "keep")
    if job["elbows"] not in ("keep", "straighten"):
        fail(f"elbows must be keep or straighten, not {job['elbows']}")
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
    return {
        "facingDeg": facing,
        "armLeftDeg": arms["left"],
        "armRightDeg": arms["right"],
        "elbowLeftDeg": elbows["left"],
        "elbowRightDeg": elbows["right"],
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
    return {"body": (body, height / 2), "shoulders": (0.5 * height, shoulders - 0.05 * height)}


def place(cam, label, view, frame, text):
    scale, z = frame["shoulders"] if view == "shoulders" else frame["body"]
    yaw = {"front": 0.0, "shoulders": 0.0, "side": 90.0, "back": 180.0}[view]
    y = math.radians(yaw)
    d = Vector((math.sin(y), -math.cos(y), 0.0))
    centre = Vector((0.0, 0.0, z))
    cam.data.ortho_scale = scale
    cam.location = centre + d * 10.0
    cam.rotation_euler = (-d).to_track_quat("-Z", "Y").to_euler()
    cam.data.clip_end = 30.0
    right = Vector((math.cos(y), math.sin(y), 0.0))
    label.data.body = text
    label.data.size = 0.03 * scale
    label.location = centre + d * 5.0 - right * 0.46 * scale + Vector((0.0, 0.0, 0.44 * scale))
    label.rotation_euler = (math.radians(90.0), 0.0, y)


def render_sheet(rig, out, title, frame):
    cam, label = setup_scene()
    cells_dir = out + "_cells"
    os.makedirs(cells_dir, exist_ok=True)
    sheet = np.ones((CELL * len(SHEET_POSES), CELL * len(SHEET_VIEWS), 4), np.float32)
    for ri, pose in enumerate(SHEET_POSES):
        rig.reset_pose()
        sheet_pose(rig, pose)
        for ci, view in enumerate(SHEET_VIEWS):
            place(cam, label, view, frame, f"{title}  {pose}  {view}")
            path = os.path.join(cells_dir, f"{pose}_{view}.png")
            bpy.context.scene.render.filepath = path
            bpy.ops.render.render(write_still=True)
            img = bpy.data.images.load(path)
            px = np.array(img.pixels[:], np.float32).reshape(CELL, CELL, 4)[::-1]
            sheet[ri * CELL:(ri + 1) * CELL, ci * CELL:(ci + 1) * CELL] = px
            bpy.data.images.remove(img)
            if ci:
                sheet[ri * CELL:(ri + 1) * CELL, ci * CELL:ci * CELL + 2, :3] = 0.15
        if ri:
            sheet[ri * CELL:ri * CELL + 2, :, :3] = 0.15
    rig.reset_pose()
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
    lifted = level_arms(rig, job["elbows"])
    apply_as_rest(rig)
    weights = weight_checks(rig, fix=True)
    rig.reset_pose()
    after = measure(rig)
    if "after" in job["sheets"]:
        render_sheet(rig, job["sheets"]["after"], "after", frame)
    export(rig, job["output"])
    report = {
        "before": before,
        "after": after,
        "turnedDeg": turned,
        "moved": moved,
        "elbows": job["elbows"],
        "lifted": lifted,
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
