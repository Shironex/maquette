"""Bake animation library clips onto a rigged model.

Maquette runs this headless, never with a window:

    blender -b --factory-startup --python-exit-code 1 --python retarget_clips.py -- \
        --job retarget.json

retarget.json names the model, where the baked copy and the report go, the
clips with their settings and bone pairs, the model bones that keep their
rest direction, the model clips the bake replaces and the model's feet:

    {"model": "in/model.glb", "out": "draft.glb", "report": "report.json",
     "clips": [{"id": "standing-dodge-left", "file": "in/clips/standing-dodge-left.glb",
                "name": "dodge_left", "start": 0, "end": 37, "loops": false,
                "rootMotion": true, "speed": 1.0,
                "pairs": [["mixamorig:Hips", "hips"], ["mixamorig:LeftArm", "upper_arm.L"]]}],
     "keepRest": ["spine", "neck"], "replace": [], "feet": ["foot.L", "foot.R"]}

How a clip is baked: the model's limbs are first turned so each points the
way the clip's skeleton points at rest (a T pose, say, against the model's
A pose), so both start from matched poses. Each frame then takes the world
rotation of every clip bone against its rest and applies that change to
the paired model bone, converted back into the bone's own space under its
posed parent. The hips also move, scaled by the model's hips height over
the clip's. Bones with no pair (fingers, a skirt, a sword socket) keep
their rest pose and follow their parents.

Each clip is written as one action on the model's armature, named as the
settings say, and the model is written back as GLB with its own clips and
the new ones. A clip that cannot be baked is reported with its error and
left out. The report measures each clip on the model: frames, how far the
root travels, how far the hips end from where they started (keys in the
wrong units leave by metres) and the worst slide of a planted foot.
"""

import argparse
import json
import math
import os
import sys
import time
import traceback

import bpy
from mathutils import Matrix, Vector

# A foot this close to its lowest point counts as planted, metres.
PLANTED_HEIGHT = 0.025
# Root travel below this reads as in place, metres.
ROOT_MIN = 0.05


def log(message):
    print(f"[retarget] {message}", flush=True)


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="retarget_clips.py")
    parser.add_argument("--job", required=True, help="the job file")
    return parser.parse_args(argv)


def rot3(m):
    return m.to_3x3().normalized()


def import_gltf(path):
    before = {
        "objects": set(bpy.data.objects),
        "actions": set(bpy.data.actions),
        "armatures": set(bpy.data.armatures),
        "meshes": set(bpy.data.meshes),
    }
    bpy.ops.import_scene.gltf(filepath=path)
    return {key: [d for d in getattr(bpy.data, key) if d not in seen] for key, seen in before.items()}


def remove_imported(found):
    bpy.data.batch_remove(found["objects"])
    bpy.data.batch_remove(found["actions"] + found["armatures"] + found["meshes"])


def armature_of(objects):
    arms = [o for o in objects if o.type == "ARMATURE"]
    if not arms:
        raise RuntimeError("it has no armature")
    return max(arms, key=lambda o: len(o.data.bones))


def action_of(arm):
    data = arm.animation_data
    if data is None:
        return None
    if data.action is not None:
        return data.action
    for track in data.nla_tracks:
        for strip in track.strips:
            if strip.action is not None:
                return strip.action
    return None


def play(arm, action):
    """Makes `action` the one `arm` plays, alone."""
    data = arm.animation_data or arm.animation_data_create()
    for track in list(data.nla_tracks):
        track.mute = True
    data.action = action
    if action.slots and data.action_slot is None:
        data.action_slot = action.slots[0]


def hierarchy_order(arm_data):
    roots = sorted([b for b in arm_data.bones if b.parent is None], key=lambda b: b.name)
    order, queue = [], list(roots)
    while queue:
        b = queue.pop(0)
        order.append(b)
        queue.extend(sorted(b.children, key=lambda c: c.name))
    return order


class Target:
    """The model's armature: its bones parents first, their rest matrices
    and which clip bone drives each."""

    def __init__(self, arm, pairs, keep_rest):
        self.arm = arm
        self.order = hierarchy_order(arm.data)
        self.mw = arm.matrix_world.copy()
        self.mw_inv = self.mw.inverted()
        self.mw_rot = rot3(self.mw).to_quaternion()
        self.mw_rot_inv = self.mw_rot.inverted()
        self.rest = {b.name: b.matrix_local.copy() for b in self.order}
        self.parent_rest_inv = {
            b.name: b.parent.matrix_local.inverted() if b.parent else None for b in self.order
        }
        self.source = {tgt: src for src, tgt in pairs if tgt in self.rest}
        self.keep_rest = set(keep_rest)
        roots = [b for b in self.order if b.name in self.source]
        if not roots:
            raise RuntimeError("no clip bone maps onto the model")
        # The hips: the driven bone nearest the root.
        self.hips = min(roots, key=lambda b: self.depth(b))
        self.hips_rest_world = self.mw @ self.hips.head_local
        for pb in arm.pose.bones:
            pb.rotation_mode = "QUATERNION"

    def depth(self, bone):
        n = 0
        while bone.parent:
            bone, n = bone.parent, n + 1
        return n

    def base(self, pose, b):
        if b.parent:
            return pose[b.parent.name] @ self.parent_rest_inv[b.name] @ self.rest[b.name]
        return self.rest[b.name].copy()

    def reference(self, src_rest_dir):
        """World rotations of the model's bones in a pose matching the clip
        skeleton's rest directions, and how far each limb was turned."""
        pose, ref_rot, angles = {}, {}, {}
        for b in self.order:
            n = b.name
            p = self.base(pose, b)
            src = self.source.get(n)
            if src in src_rest_dir and n not in self.keep_rest:
                cur = (self.mw.to_3x3() @ p.col[1].to_3d()).normalized()
                q_world = cur.rotation_difference(src_rest_dir[src])
                angles[n] = math.degrees(q_world.angle)
                q_arm = self.mw_rot_inv @ q_world @ self.mw_rot
                r = q_arm.to_matrix() @ p.to_3x3().normalized()
                p = Matrix.Translation(p.translation) @ r.to_4x4()
            pose[n] = p
            ref_rot[n] = self.mw_rot @ rot3(p).to_quaternion()
        return ref_rot, angles


class Source:
    """A clip's skeleton: rest rotations and directions per bone."""

    def __init__(self, arm, hips_name):
        self.arm = arm
        self.mw = arm.matrix_world.copy()
        self.rest_rot, self.rest_dir = {}, {}
        for b in arm.data.bones:
            m = self.mw @ b.matrix_local
            self.rest_rot[b.name] = rot3(m).to_quaternion()
            self.rest_dir[b.name] = (rot3(m) @ Vector((0.0, 1.0, 0.0))).normalized()
        if hips_name not in arm.data.bones:
            raise RuntimeError(f"the clip has no bone {hips_name}")
        self.hips_name = hips_name
        self.hips_rest_world = self.mw @ arm.data.bones[hips_name].head_local

    def sample(self):
        ev = self.arm.evaluated_get(bpy.context.evaluated_depsgraph_get())
        mw = ev.matrix_world
        rots = {pb.name: rot3(mw @ pb.matrix).to_quaternion() for pb in ev.pose.bones}
        return rots, mw @ ev.pose.bones[self.hips_name].head


def frame_times(action, clip):
    f0 = int(round(action.frame_range[0]))
    first, last = f0 + clip["start"], f0 + clip["end"]
    speed = max(float(clip["speed"]), 0.01)
    count = int(math.floor((last - first) / speed + 1e-6)) + 1
    return [first + i * speed for i in range(count)]


def set_frame(scene, t):
    whole = int(math.floor(t))
    scene.frame_set(whole, subframe=t - whole)


def slide(tracks):
    """The worst horizontal drift of a planted foot: (cm, from, to, side)."""
    worst = (0.0, 0, 0, "left")
    for side, track in tracks.items():
        if len(track) < 3:
            continue
        floor = min(p.z for p in track)
        run = []
        for i, p in enumerate(track + [None]):
            if p is not None and p.z <= floor + PLANTED_HEIGHT:
                run.append(i)
                continue
            if len(run) >= 3:
                start = track[run[0]]
                drift = max(math.hypot(track[j].x - start.x, track[j].y - start.y) for j in run)
                if drift * 100.0 > worst[0]:
                    worst = (drift * 100.0, run[0], run[-1], side)
            run = []
    return worst


def write_action(name, tgt, times, rot_samples, loc_samples):
    action = bpy.data.actions.new(name)
    if action.name != name:
        raise RuntimeError(f"the model already has an action named {name}")
    slot = action.slots.new(id_type="OBJECT", name=tgt.arm.name)
    strip = action.layers.new("Layer").strips.new(type="KEYFRAME")
    bag = strip.channelbags.new(slot)
    frames = list(range(len(times)))

    def write(data_path, index, group, values):
        fc = bag.fcurves.new(data_path, index=index, group_name=group)
        n = len(values)
        fc.keyframe_points.add(n)
        co = [0.0] * (2 * n)
        co[0::2] = frames[:n]
        co[1::2] = values
        fc.keyframe_points.foreach_set("co", co)
        fc.keyframe_points.foreach_set("interpolation", [1] * n)  # LINEAR
        fc.update()

    for b in tgt.order:
        path = f'pose.bones["{b.name}"].rotation_quaternion'
        if b.name in rot_samples:
            for i in range(4):
                write(path, i, b.name, [q[i] for q in rot_samples[b.name]])
        else:
            for i, v in enumerate((1.0, 0.0, 0.0, 0.0)):
                write(path, i, b.name, [v])
    hips = f'pose.bones["{tgt.hips.name}"].location'
    for i in range(3):
        write(hips, i, tgt.hips.name, [v[i] for v in loc_samples])
    action.use_fake_user = True
    action.use_frame_range = True
    action.frame_start = 0
    action.frame_end = max(len(times) - 1, 1)
    return action, slot


def bake(scene, tgt, src, action, clip, feet):
    ref_rot, angles = tgt.reference(src.rest_dir)
    src_ref_inv = {k: q.inverted() for k, q in src.rest_rot.items()}
    scale = tgt.hips_rest_world.z / src.hips_rest_world.z if src.hips_rest_world.z > 1e-4 else 1.0
    driven = [b.name for b in tgt.order if tgt.source.get(b.name) in src.rest_rot]
    rot_samples = {n: [] for n in driven}
    loc_samples, hips_track, prev_q = [], [], {}
    foot_tracks = {side: [] for side in feet}
    times = frame_times(action, clip)
    for t in times:
        set_frame(scene, t)
        srot, src_hips = src.sample()
        delta = (src_hips - src.hips_rest_world) * scale
        removed = Vector((0.0, 0.0, 0.0))
        if not clip["rootMotion"]:
            removed = Vector((delta.x, delta.y, 0.0))
            delta = delta - removed
        pose = {}
        for b in tgt.order:
            n = b.name
            base = tgt.base(pose, b)
            if n not in rot_samples:
                pose[n] = base
                continue
            k = tgt.source[n]
            r_world = srot[k] @ src_ref_inv[k] @ ref_rot[n]
            r_arm = (tgt.mw_rot_inv @ r_world).to_matrix().to_4x4()
            if b == tgt.hips:
                head_world = tgt.hips_rest_world + delta
                hips_track.append(head_world + removed)
                head = tgt.mw_inv @ head_world
            else:
                head = base.translation
            p = Matrix.Translation(head) @ r_arm
            q = (base.inverted() @ p).to_quaternion().normalized()
            if n in prev_q and prev_q[n].dot(q) < 0.0:
                q.negate()
            prev_q[n] = q
            rot_samples[n].append(q)
            if b == tgt.hips:
                loc_samples.append((base.inverted() @ p).translation.copy())
            pose[n] = p
        for side, bone in feet.items():
            if bone in pose:
                foot_tracks[side].append(tgt.mw @ pose[bone].translation + removed)
    if not loc_samples:
        loc_samples = [Vector((0.0, 0.0, 0.0))]
    made, slot = write_action(clip["name"], tgt, times, rot_samples, loc_samples)
    first, last = hips_track[0], hips_track[-1]
    travel = math.hypot(last.x - first.x, last.y - first.y) if clip["rootMotion"] else 0.0
    farthest = max((h - first).length for h in hips_track)
    cm, slide_from, slide_to, foot = slide(foot_tracks)
    return made, slot, {
        "frames": len(times),
        "travel": round(travel, 4),
        "farthest": round(farthest, 3),
        "slideCm": round(cm, 2),
        "slideFrom": slide_from,
        "slideTo": slide_to,
        "foot": foot,
        "driven": len(driven),
        "aligned": max(angles.values(), default=0.0),
        "hipsScale": scale,
    }


def stash(arm, action, slot):
    """Keeps `action` on the model as an NLA strip, so the export writes it."""
    data = arm.animation_data or arm.animation_data_create()
    track = data.nla_tracks.new()
    track.name = action.name
    strip = track.strips.new(action.name, int(action.frame_start), action)
    if hasattr(strip, "action_slot"):
        strip.action_slot = slot
    track.mute = True


def main():
    args = parse_args()
    started = time.time()
    with open(args.job, encoding="utf-8") as f:
        job = json.load(f)
    base = os.path.dirname(os.path.abspath(args.job))
    at = lambda rel: os.path.join(base, rel)  # noqa: E731

    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    model = import_gltf(at(job["model"]))
    target_arm = armature_of(model["objects"])
    for name in job.get("replace", []):
        old = bpy.data.actions.get(name)
        if old is not None:
            bpy.data.actions.remove(old)
    keep = set(bpy.data.actions)
    model_objects = list(model["objects"])
    sides = ("left", "right")
    feet = {side: bone for side, bone in zip(sides, job.get("feet", []))}

    report = {"clips": [], "driven": 0, "alignedDeg": 0.0, "hipsScale": 1.0}
    for clip in job["clips"]:
        entry = {"id": clip["id"], "name": clip["name"], "error": None}
        found = None
        try:
            found = import_gltf(at(clip["file"]))
            src_arm = armature_of(found["objects"])
            action = action_of(src_arm)
            if action is None:
                raise RuntimeError("the clip has no keys")
            play(src_arm, action)
            hips = next((s for s, t in clip["pairs"] if t and s), None)
            if hips is None:
                raise RuntimeError("no clip bone maps onto the model")
            tgt = Target(target_arm, clip["pairs"], job.get("keepRest", []))
            hips = next(s for s, t in clip["pairs"] if t == tgt.hips.name)
            src = Source(src_arm, hips)
            made, slot, info = bake(scene, tgt, src, action, clip, feet)
            stash(target_arm, made, slot)
            keep.add(made)
            report["driven"] = max(report["driven"], info.pop("driven"))
            report["alignedDeg"] = round(max(report["alignedDeg"], info.pop("aligned")), 2)
            report["hipsScale"] = round(info.pop("hipsScale"), 4)
            entry.update(info)
            log(f"baked {clip['name']}, {info['frames']} frames")
        except Exception as err:  # noqa: BLE001 - one clip failing leaves the others
            traceback.print_exc()
            entry["error"] = str(err)
            log(f"could not bake {clip['name']}: {err}")
        finally:
            if found is not None:
                remove_imported({**found, "actions": [a for a in found["actions"] if a not in keep]})
        report["clips"].append(entry)

    data = target_arm.animation_data
    if data is not None:
        data.action = None
    bpy.ops.object.select_all(action="DESELECT")
    for obj in model_objects:
        if obj.name in bpy.data.objects:
            obj.select_set(True)
    bpy.ops.export_scene.gltf(
        filepath=at(job["out"]),
        export_format="GLB",
        use_selection=True,
        export_animations=True,
        export_animation_mode="ACTIONS",
        export_yup=True,
    )
    report["seconds"] = round(time.time() - started, 2)
    with open(at(job["report"]), "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2)
    log(f"wrote {job['out']}")


main()
