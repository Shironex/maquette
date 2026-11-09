"""Bake one animation recipe onto a rigged model, as one new clip.

Maquette runs this headless, never with a window:

    blender -b --factory-startup --python-exit-code 1 --python bake_recipe.py -- \
        --job recipe.json

retarget_clips.py and surface.py sit next to it. recipe.json holds the
retarget job of the recipe's base clip (as retarget_clips.py reads it), the
recipe, and where the model, the report and the progress go:

    {"retarget": {...one clip...},
     "recipe": {"clip": "standing-idle", "hips": "hips",
                "bones": {"rightHand": "hand.R", "rightLowerArm": "forearm.R", ...},
                "layers": [{"id": "l-1", "kind": "reach", "bone": "rightHand",
                            "settings": {"weight": 1, "start": 0.2, "end": 0.6,
                                         "target": "lantern", ...}}],
                "targets": {"lantern": {"slot": 0}, "spot": {"point": [0, 1.2, 0.4]},
                            "cup": {"prop": "in/props/cup.glb",
                                    "propMatrix": [16 numbers], "modelMatrix": [16 numbers]}},
                "stretchFrames": 10},
     "out": "staged.glb", "report": "result.json", "progress": "progress.json"}

A recipe on the rest pose has no base clip: "retarget" then lists no clips
and the recipe holds "clip": null and "rest": {"name": "wave_hello",
"frames": 60, "fps": 30}. The model is still imported through the retarget
and the rest pose is held for those frames (keys on frames 0 to 60), keyed as a clip of its own, so
the changes are laid over it as over any clip and its feet measure the rest
pose's (still) feet.

How it bakes: the base clip goes onto the model through the retarget, as
Use on model bakes it, under the clip's new name. Then, frame by frame, the
recipe's changes are laid over it in order, each eased in and out over its
part of the clip as the rough preview eases it (a reach in over the first
half of its range and held to its end, the others over a quarter each way):

- nudge turns the bone about the model's own axes (X to its left, Y up, Z
  ahead) and moves it along them, on top of the clip;
- wave swings the bone about the axis the model faces along, on a sine;
- look at turns the head or neck the shortest way toward its target, at
  most 80 degrees;
- reach puts an IK constraint on the hand or foot, aimed at an empty on its
  target, up through its upper arm or leg, its influence keyed by the ease;
  a limb the clip holds dead straight is first bent a few degrees the way
  an elbow or knee bends, so the solver knows which way to fold it;
- pins come last, after every other change (recipe_pins.py): a hand or foot
  held on its spot (where it is at the pin's first frame, or a blank's
  target) by IK, a foot kept flat and a hand kept turned as it was.

Targets sit in the model's own frame, read from the rig as the preview reads
it: the model faces up crossed with the line from its left upper arm to its
right one (or its upper legs). A point is where it says, from the spot the
model stands on. A prop is where the scene layout places it, seen from where
the layout places the model, when both are placed; otherwise it takes the
preview's slot in front of the model, within reach of a hand.

Each frame's posed bones, constraints included, are read back into their
own channels (visual keying), the constraints and empties are removed, and
the clip is written again from those keys. The model is written as GLB with
its own clips and this one. The report measures what the review needs: each
foot's world position per frame (the base clip's and the recipe's), the
rig's worst stretch past a healthy skin on a few frames (surface.py), and
per reach the gap to its target at its end and the first frame its elbow or
knee bends the other way than the clip has it, and per pin how far its hand
or foot drifts from its spot while it holds whole.
"""

import argparse
import importlib.util
import json
import math
import os
import sys
import time
import traceback

import bpy
from mathutils import Matrix, Quaternion, Vector

# The share of a change's range it eases in over, and out over.
RAMP = 0.25
# A reach eases in over this share of its range, then holds.
REACH_IN = 0.5
# How far look at turns a bone.
MAX_LOOK = math.radians(80.0)
# A limb is held straight within this angle, and is bent this far before
# its reach is solved.
STRAIGHT = math.radians(3.0)
PREBEND = math.radians(10.0)
UP = Vector((0.0, 0.0, 1.0))
# What a model faces without a pair of arms or legs to read: glTF's +Z.
DEFAULT_AHEAD = Vector((0.0, -1.0, 0.0))
# The height a model without a mesh is taken to have, metres.
DEFAULT_HEIGHT = 1.7
# A reach's end bone, with the upper and lower bones its IK solves.
LIMBS = {
    "leftHand": ("leftUpperArm", "leftLowerArm"),
    "rightHand": ("rightUpperArm", "rightLowerArm"),
    "leftFoot": ("leftUpperLeg", "leftLowerLeg"),
    "rightFoot": ("rightUpperLeg", "rightLowerLeg"),
}


def log(message):
    print(f"[recipe] {message}", flush=True)


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="bake_recipe.py")
    parser.add_argument("--job", required=True, help="the job file")
    return parser.parse_args(argv)


def load_sibling(folder, name):
    spec = importlib.util.spec_from_file_location(name, os.path.join(folder, f"{name}.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class Progress:
    """Writes how far the bake is to a small file Maquette reads while it runs."""

    def __init__(self, path):
        self.path = path

    def set(self, label, done, total):
        tmp = self.path + ".tmp"
        with open(tmp, "w", encoding="utf-8") as f:
            json.dump({"label": label, "done": done, "total": total}, f)
        os.replace(tmp, self.path)
        print(f"[recipe] progress {done}/{total}", flush=True)


def smooth(x):
    if x <= 0.0:
        return 0.0
    if x >= 1.0:
        return 1.0
    return x * x * (3.0 - 2.0 * x)


def envelope(kind, start, end, share):
    """How much of a change shows at `share` of the clip, 0 to 1."""
    if not end > start or share < start or share > end:
        return 0.0
    x = (share - start) / (end - start)
    if kind == "reach":
        return smooth(x / REACH_IN)
    return min(smooth(x / RAMP), smooth((1.0 - x) / RAMP))


def num(value, default=0.0):
    return float(value) if isinstance(value, (int, float)) and math.isfinite(value) else default


def matrix_of(column_major):
    m = column_major
    return Matrix([[m[c * 4 + r] for c in range(4)] for r in range(4)])


def to_gltf(v):
    return Vector((v.x, v.z, -v.y))


def from_gltf(v):
    return Vector((v.x, -v.z, v.y))


class Rig:
    """The model's armature, its humanoid bones by VRM name and its own frame."""

    def __init__(self, arm, bones):
        self.arm = arm
        self.names = bones
        self.mw = arm.matrix_world.copy()
        self.mw_rot = self.mw.to_3x3().normalized().to_quaternion()
        self.mw_rot_inv = self.mw_rot.inverted()
        self.ahead = self.read_facing()
        self.left = UP.cross(self.ahead).normalized()
        self.basis = Matrix((self.left, UP, self.ahead)).transposed().to_quaternion()

    def pose_bone(self, vrm):
        name = self.names.get(vrm)
        return self.arm.pose.bones.get(name) if name else None

    def rest_head(self, vrm):
        name = self.names.get(vrm)
        bone = self.arm.data.bones.get(name) if name else None
        return self.mw @ bone.head_local if bone else None

    def head(self, pb):
        return self.mw @ pb.head

    def world_rotation(self, pb):
        return (self.mw @ pb.matrix).to_3x3().normalized().to_quaternion()

    def read_facing(self):
        """Up crossed with the left to right upper arm (else upper leg), level."""
        for left, right in (("leftUpperArm", "rightUpperArm"), ("leftUpperLeg", "rightUpperLeg")):
            a, b = self.rest_head(left), self.rest_head(right)
            if a is None or b is None:
                continue
            ahead = UP.cross(b - a)
            ahead.z = 0.0
            if ahead.length > 1e-4:
                return ahead.normalized()
        return DEFAULT_AHEAD.copy()

    def place(self, x, y, z):
        return self.left * x + UP * y + self.ahead * z

    def model_turn(self, q):
        """`q`, a turn about the model's own axes, as a turn in the world."""
        return self.basis @ q @ self.basis.inverted()

    def turn(self, pb, q_world):
        """Turns `pb` about its head by `q_world`, under its parent as posed."""
        q_arm = self.mw_rot_inv @ q_world @ self.mw_rot
        m = pb.matrix.copy()
        head = m.translation.copy()
        pb.matrix = Matrix.Translation(head) @ q_arm.to_matrix().to_4x4() @ Matrix.Translation(-head) @ m

    def move(self, pb, world):
        m = pb.matrix.copy()
        m.translation = m.translation + self.mw_rot_inv @ world
        pb.matrix = m


def body_meshes(objects):
    """The meshes a file really holds. Blender's glTF import also makes a
    mesh (an icosphere about 2 m across) to draw the bones with; it is no
    part of the model and is never written, but counting it made a 1.5 m
    model 2.5 m tall, so the slot sat above its head where the rough
    preview, which never sees it, put it at chest height."""
    shapes = {
        pb.custom_shape
        for o in objects
        if o.type == "ARMATURE" and o.pose
        for pb in o.pose.bones
        if pb.custom_shape is not None
    }
    return [o for o in objects if o.type == "MESH" and o not in shapes]


def model_height(objects, arm):
    zs = [(o.matrix_world @ Vector(c)).z for o in body_meshes(objects) for c in o.bound_box]
    if not zs:
        zs = [(arm.matrix_world @ p).z for b in arm.data.bones for p in (b.head_local, b.tail_local)]
    height = max(zs) - min(zs) if zs else 0.0
    return height if height > 1e-3 else DEFAULT_HEIGHT


def prop_center(rt, path):
    """The middle of a prop's bounds as its file places it, Blender axes."""
    found = rt.import_gltf(path)
    try:
        points = [o.matrix_world @ Vector(c) for o in body_meshes(found["objects"]) for c in o.bound_box]
        if not points:
            return Vector((0.0, 0.0, 0.0))
        low = Vector(tuple(min(p[k] for p in points) for k in range(3)))
        high = Vector(tuple(max(p[k] for p in points) for k in range(3)))
        return (low + high) / 2
    finally:
        rt.remove_imported(found)


def targets_of(rt, rig, specs, height, at):
    out = {}
    for blank, spec in specs.items():
        if "point" in spec:
            out[blank] = rig.place(*(num(v) for v in spec["point"]))
        elif "prop" in spec:
            center = to_gltf(prop_center(rt, at(spec["prop"])))
            world = matrix_of(spec["propMatrix"]) @ center
            out[blank] = from_gltf(matrix_of(spec["modelMatrix"]).inverted() @ world)
        elif "slot" in spec:
            n = int(spec["slot"])
            side = -1.0 if n % 2 == 0 else 1.0
            row = n // 2
            out[blank] = rig.place(side * 0.22 * height, 0.65 * height, (0.18 + 0.1 * row) * height)
    return out


def chain_length(arm, end, upper):
    """How many bones an IK on `end` (its head) solves to reach up to `upper`."""
    count, bone = 0, arm.data.bones[end].parent
    while bone is not None:
        count += 1
        if bone.name == upper:
            return count
        bone = bone.parent
    raise RuntimeError(f"{end} does not hang under {upper}, so its reach cannot be solved")


class Reach:
    """One reach layer: its IK constraint on an empty at the target, and what is measured."""

    def __init__(self, rig, layer, target, scene):
        self.layer = layer
        end_vrm = layer["bone"]
        upper_vrm, lower_vrm = LIMBS[end_vrm]
        self.end, self.upper, self.lower = (rig.pose_bone(v) for v in (end_vrm, upper_vrm, lower_vrm))
        if not (self.end and self.upper and self.lower):
            raise RuntimeError(f"the model has no {end_vrm} limb to reach with")
        self.target = target
        self.empty = bpy.data.objects.new(f"recipe_target_{layer['id']}", None)
        scene.collection.objects.link(self.empty)
        self.empty.location = target
        self.constraint = self.end.constraints.new("IK")
        self.constraint.name = f"recipe_{layer['id']}"
        self.constraint.target = self.empty
        self.constraint.use_tail = False
        self.constraint.chain_count = chain_length(rig.arm, self.end.name, self.upper.name)
        self.constraint.influence = 0.0
        self.gap = None
        self.gap_frame = None
        self.flip_frame = None

    def bend(self, rig):
        root, mid, end = rig.head(self.upper), rig.head(self.lower), rig.head(self.end)
        return (mid - root).cross(end - root)

    def prebend(self, rig, influence):
        """Bends a limb the clip holds dead straight a few degrees the way an
        elbow (the forearm forward) or a knee (the shin back) bends: the
        solver cannot tell which way to fold a straight chain and leaves it
        reaching past its target. Scaled by the influence, so it blends out
        with the reach."""
        root, mid, end = rig.head(self.upper), rig.head(self.lower), rig.head(self.end)
        upper, lower = mid - root, end - mid
        if upper.length < 1e-6 or lower.length < 1e-6:
            return
        if upper.normalized().cross(lower.normalized()).length > math.sin(STRAIGHT):
            return
        way = rig.ahead if self.layer["bone"].endswith("Hand") else -rig.ahead
        axis = lower.cross(way)
        if axis.length < 1e-6:
            axis = lower.cross(rig.left)
        rig.turn(self.lower, Quaternion(axis.normalized(), PREBEND * influence))

    def remove(self):
        self.end.constraints.remove(self.constraint)
        bpy.data.objects.remove(self.empty)


def lay_over(rig, layer, amount, share, target):
    """Lays one nudge, wave or look at over the pose as it stands."""
    s = layer["settings"]
    pb = rig.pose_bone(layer["bone"])
    if pb is None:
        return
    kind = layer["kind"]
    if kind == "nudge":
        rotation = s.get("rotation")
        if rotation:
            x, y, z = (math.radians(num(v)) for v in rotation)
            turn = (Matrix.Rotation(x, 3, "X") @ Matrix.Rotation(y, 3, "Y") @ Matrix.Rotation(z, 3, "Z")).to_quaternion()
            rig.turn(pb, rig.model_turn(Quaternion().slerp(turn, amount)))
            bpy.context.view_layer.update()
        offset = s.get("offset")
        if offset:
            rig.move(pb, rig.place(*(num(v) for v in offset)) * amount)
    elif kind == "wave":
        start, end = num(s.get("start")), num(s.get("end"), 1.0)
        x = (share - start) / (end - start) if end > start else 0.0
        angle = math.radians(num(s.get("amount"))) * math.sin(2.0 * math.pi * num(s.get("cycles")) * x) * amount
        rig.turn(pb, Quaternion(rig.ahead, angle))
    elif kind == "look-at" and target is not None:
        facing = rig.world_rotation(pb) @ rig.facings[pb.name]
        want = target - rig.head(pb)
        if want.length < 1e-6 or facing.length < 1e-6:
            return
        turn = facing.rotation_difference(want)
        angle = turn.angle
        limit = MAX_LOOK / angle if angle > MAX_LOOK else 1.0
        rig.turn(pb, Quaternion().slerp(turn, amount * limit))


def skinned_mesh(objects, arm):
    meshes = [
        o
        for o in objects
        if o.type == "MESH" and any(m.type == "ARMATURE" and m.object == arm for m in o.modifiers)
    ]
    return max(meshes, key=lambda o: len(o.data.vertices), default=None)


def stretch_reader(folder, objects, arm):
    """A function measuring the pose's stretch past a healthy skin in mm, or None."""
    mesh = skinned_mesh(objects, arm)
    if mesh is None:
        return None, "the model has no skinned mesh to measure"
    try:
        surface = load_sibling(folder, "surface")
        reading = surface.Surface(arm, mesh)
    except Exception as err:  # noqa: BLE001 - the stretch is a number, not the bake
        traceback.print_exc()
        return None, f"the stretch could not be measured: {err}"
    return lambda: reading.measure()["beyond_body_mm"], None


def sample_frames(count, wanted, extra):
    if count <= 0:
        return set()
    step = max((count - 1) / max(wanted - 1, 1), 1.0)
    picked = {min(int(round(i * step)), count - 1) for i in range(wanted)}
    return picked | {f for f in extra if 0 <= f < count}


def end_frame(layer, count, shares=None):
    """The last frame of a change: where its end share falls. A recipe that
    joins clips or edits its base says each frame's share."""
    end = num(layer["settings"].get("end"), 1.0)
    if shares:
        return max([i for i, share in enumerate(shares) if share <= end + 1e-6], default=0)
    return max(0, min(count - 1, int(math.floor(end * (count - 1) + 1e-6))))


def rest_action(rt, arm, hips, frames):
    """An action that holds the rest pose for `frames` frames, as a base clip
    would: keys on frame 0 to `frames`, the way the library counts a clip."""
    keys = Keys(arm, rt.hierarchy_order(arm.data), hips)
    times = [0.0, float(max(frames, 1))]
    still = Quaternion((1.0, 0.0, 0.0, 0.0))
    rot = {b.name: [still.copy(), still.copy()] for b in keys.order}
    loc = [Vector((0.0, 0.0, 0.0)), Vector((0.0, 0.0, 0.0))]
    action, _ = rt.write_action("recipe_rest_pose", keys, times, rot, loc, {})
    rt.play(arm, action)
    return action


def breathe(rig, angle):
    """Tips the chest, and half as much the spine, about the model's left axis."""
    for vrm, share in (("chest", 1.0), ("spine", 0.5)):
        pb = rig.pose_bone(vrm)
        if pb is not None:
            rig.turn(pb, Quaternion(rig.left, angle * share))
            bpy.context.view_layer.update()


def solve(rt, scene, arm, objects, action, job, at, progress):
    recipe = job["recipe"]
    clip_spec = job["clipSpec"]
    rig = Rig(arm, recipe["bones"])
    height = model_height(objects, arm)
    targets = targets_of(rt, rig, recipe.get("targets", {}), height, at)
    rig.facings = {}
    for vrm in ("head", "neck"):
        pb = rig.pose_bone(vrm)
        if pb is not None:
            rest = (rig.mw @ pb.bone.matrix_local).to_3x3().normalized().to_quaternion()
            rig.facings[pb.name] = rest.inverted() @ rig.ahead
    layers = recipe["layers"]
    reaches = {}
    for layer in layers:
        if layer["kind"] != "reach":
            continue
        target = targets.get(layer["settings"].get("target"))
        if target is None:
            raise RuntimeError(f"the reach {layer['id']} has no target")
        reaches[layer["id"]] = Reach(rig, layer, target, scene)
    folder = os.path.dirname(at(job["out"]))
    pins = load_sibling(folder, "recipe_pins").Pins(rig, recipe.get("pins") or [], targets, scene)

    rt.play(arm, action)
    count = int(job["frames"])
    # A built motion keeps the model's rate, which can differ from its own.
    times = rt.key_frames(count, clip_spec, job.get("sceneFps") or rt.clip_fps(clip_spec))
    order = rt.hierarchy_order(arm.data)
    measure, stretch_note = stretch_reader(folder, objects, arm)
    shares = job.get("shares")
    breaths = job.get("breaths")
    ends = {key: end_frame(r.layer, count, shares) for key, r in reaches.items()}
    sampled = sample_frames(count, int(recipe.get("stretchFrames", 10)), list(ends.values()))
    rot_samples = {b.name: [] for b in order}
    loc_samples = {b.name: [] for b in order}
    prev_q = {}
    feet_bones = job["retarget"].get("feet", [])
    sides = dict(zip(("left", "right"), feet_bones))
    base_feet = {side: [] for side in sides}
    feet = {side: [] for side in sides}
    stretch = {"mm": None, "frame": None, "base": None}
    for i, t in enumerate(times):
        for pb in arm.pose.bones:
            pb.location = (0.0, 0.0, 0.0)
            pb.rotation_quaternion = (1.0, 0.0, 0.0, 0.0)
        for reach in reaches.values():
            reach.constraint.influence = 0.0
        pins.reset()
        rt.set_frame(scene, t)
        bpy.context.view_layer.update()
        share = shares[i] if shares else (i / (count - 1) if count > 1 else 0.0)
        if breaths and breaths[i]:
            breathe(rig, breaths[i])
        for side, bone in sides.items():
            base_feet[side].append(rig.head(arm.pose.bones[bone]))
        bends = {key: reach.bend(rig) for key, reach in reaches.items()}
        if measure is not None and i in sampled:
            base = measure()
            stretch["base"] = base if stretch["base"] is None else max(stretch["base"], base)
        for layer in layers:
            s = layer["settings"]
            amount = num(s.get("weight"), 1.0) * envelope(layer["kind"], num(s.get("start")), num(s.get("end"), 1.0), share)
            if amount <= 0.0:
                continue
            if layer["kind"] == "reach":
                reach = reaches[layer["id"]]
                reach.constraint.influence = min(amount, 1.0)
                reach.prebend(rig, min(amount, 1.0))
                bpy.context.view_layer.update()
                continue
            lay_over(rig, layer, amount, share, targets.get(s.get("target")))
            bpy.context.view_layer.update()
        bpy.context.view_layer.update()
        pins.hold(rig, i)
        bpy.context.view_layer.update()
        for b in order:
            pb = arm.pose.bones[b.name]
            basis = arm.convert_space(pose_bone=pb, matrix=pb.matrix, from_space="POSE", to_space="LOCAL")
            loc, q, _ = basis.decompose()
            q.normalize()
            if b.name in prev_q and prev_q[b.name].dot(q) < 0.0:
                q.negate()
            prev_q[b.name] = q
            rot_samples[b.name].append(q)
            loc_samples[b.name].append(loc)
        for side, bone in sides.items():
            feet[side].append(rig.head(arm.pose.bones[bone]))
        pins.measure(rig, i)
        for key, reach in reaches.items():
            s = reach.layer["settings"]
            if i == ends[key]:
                reach.gap = (rig.head(reach.end) - reach.target).length
                reach.gap_frame = i
            on = envelope("reach", num(s.get("start")), num(s.get("end"), 1.0), share) > 0.0
            after = reach.bend(rig)
            before = bends[key]
            flipped = before.length_squared > 1e-10 and after.length_squared > 1e-10 and before.dot(after) < 0.0
            if on and flipped and reach.flip_frame is None:
                reach.flip_frame = i
        if measure is not None and i in sampled:
            mm = measure()
            if stretch["mm"] is None or mm > stretch["mm"]:
                stretch["mm"], stretch["frame"] = mm, i
        if i % 5 == 0 or i == count - 1:
            progress.set(f"Solving frame {i + 1} of {count}", i + 1, count)

    for reach in reaches.values():
        reach.remove()
    pins.remove()
    return rig, height, {
        "rot": rot_samples,
        "loc": loc_samples,
        "times": times,
        "feet": feet,
        "baseFeet": base_feet,
        "stretch": stretch,
        "stretchNote": stretch_note,
        "reaches": [
            {
                "layer": r.layer["id"],
                "bone": r.layer["bone"],
                "gap": r.gap,
                "frame": r.gap_frame,
                "flipFrame": r.flip_frame,
            }
            for r in reaches.values()
        ],
        "pins": pins.report(),
    }


class Keys:
    """What retarget_clips.write_action needs of the model's armature."""

    def __init__(self, arm, order, hips):
        self.arm = arm
        self.order = order
        self.hips = arm.data.bones[hips]


def write_clip(rt, arm, action, name, hips, solved):
    """Writes the solved keys as the clip `name`, in place of the base clip's action."""
    data = arm.animation_data
    for track in list(data.nla_tracks):
        if any(strip.action == action for strip in track.strips):
            data.nla_tracks.remove(track)
    data.action = None
    bpy.data.actions.remove(action)
    keys = Keys(arm, rt.hierarchy_order(arm.data), hips)
    moved = {
        bone: [v.copy() for v in locs]
        for bone, locs in solved["loc"].items()
        if bone != hips and any(v.length > 1e-6 for v in locs)
    }
    made, slot = rt.write_action(name, keys, solved["times"], solved["rot"], solved["loc"][hips], moved)
    rt.stash(arm, made, slot)


def track_of(points):
    return [[round(c, 4) for c in p] for p in points]


def corrected(base_track, base, final):
    """The retarget's foot track (which keeps any travel taken out) moved as the recipe moved the foot."""
    out = []
    for i, p in enumerate(base_track):
        shift = final[i] - base[i] if i < len(final) else Vector((0.0, 0.0, 0.0))
        out.append([round(p[k] + shift[k], 4) for k in range(3)])
    return out


def composed(rt, scene, arm, job, made, folder):
    """The recipe's motion built from its clips as its timeline says, as one
    action, with each frame's share and breath set on the job for solve. The
    clips' own actions are removed, so the model is written without them."""
    recipe = job["recipe"]
    timeline = recipe["timeline"]
    motion = load_sibling(folder, "recipe_motion")
    rig = Rig(arm, recipe["bones"])
    order = rt.hierarchy_order(arm.data)
    names = [b.name for b in order]
    rest = recipe.get("rest")
    if rest:
        rt.set_fps(scene, float(rest.get("fps") or rt.DEFAULT_FPS))
    scene_fps = scene.render.fps / scene.render.fps_base
    parts = []
    for spec in recipe["parts"]:
        action = made.get(spec["clip"]) if spec.get("clip") else None
        if spec.get("clip") and action is None:
            raise RuntimeError(f"the clip {spec['clip']} could not go on the model")
        parts.append(motion.Part(action, scene_fps / rt.clip_fps(spec), names))
    sk = motion.Skeleton(arm, order, recipe["hips"], rig)
    poses, breaths, moved = motion.build(sk, parts, timeline, recipe.get("mirrorPairs", []))
    for action in set(made.values()):
        data = arm.animation_data
        for track in list(data.nla_tracks):
            if any(strip.action == action for strip in track.strips):
                data.nla_tracks.remove(track)
        if data.action == action:
            data.action = None
        bpy.data.actions.remove(action)
    fps = float(timeline["fps"])
    times = [t * scene_fps / fps for t in range(len(poses))]
    rot = {n: [] for n in names}
    for pose in poses:
        for n in names:
            q = pose[n][0].copy()
            if rot[n] and rot[n][-1].dot(q) < 0.0:
                q.negate()
            rot[n].append(q)
    hips = recipe["hips"]
    locs = {n: [pose[n][1].copy() for pose in poses] for n in moved | {hips}}
    keys = Keys(arm, order, hips)
    others = {n: v for n, v in locs.items() if n != hips and any(x.length > 1e-6 for x in v)}
    action, _ = rt.write_action("maquette_recipe_motion", keys, times, rot, locs[hips], others)
    rt.play(arm, action)
    job["clipSpec"] = {"fps": fps}
    job["sceneFps"] = scene_fps
    job["shares"] = [k["share"] for k in timeline["keys"]]
    job["breaths"] = breaths
    return action, {"frames": len(poses), "travel": 0.0}, recipe["name"]


def main():
    args = parse_args()
    started = time.time()
    with open(args.job, encoding="utf-8") as f:
        job = json.load(f)
    folder = os.path.dirname(os.path.abspath(args.job))
    at = lambda rel: os.path.join(folder, rel)  # noqa: E731
    progress = Progress(at(job["progress"]))
    try:
        progress.set("Putting the base clip on the model", 0, 1)
        rt = load_sibling(folder, "retarget_clips")
        scene, arm, objects, report, made = rt.retarget(job["retarget"], at)
        recipe = job["recipe"]
        rest = recipe.get("rest")
        if recipe.get("timeline"):
            action, entry, name = composed(rt, scene, arm, job, made, folder)
        elif rest:
            fps = float(rest.get("fps") or rt.DEFAULT_FPS)
            rt.set_fps(scene, fps)
            frames = max(int(rest["frames"]), 2)
            # One key more than frames: a clip of N frames has keys 0 to N.
            entry = {"frames": frames + 1, "travel": 0.0}
            action = rest_action(rt, arm, recipe["hips"], frames)
            name = rest["name"]
            job["clipSpec"] = {"fps": fps}
        else:
            entry = next(c for c in report["clips"] if c["id"] == recipe["clip"])
            if entry.get("error"):
                raise RuntimeError(f"the base clip could not go on the model: {entry['error']}")
            action = made[recipe["clip"]]
            name = job["retarget"]["clips"][0]["name"]
            job["clipSpec"] = job["retarget"]["clips"][0]
        job["frames"] = entry["frames"]
        rig, height, solved = solve(rt, scene, arm, objects, action, job, at, progress)
        progress.set("Writing the model", 0, 1)
        write_clip(rt, arm, action, name, recipe["hips"], solved)
        rt.export(at(job["out"]), arm, objects)
        base_track = entry.get("feet") or {side: track_of(points) for side, points in solved["baseFeet"].items()}
        result = {
            "clip": name,
            # The clip's length in frames as the library counts it (the last
            # key's frame, one less than the keys), so it matches its base.
            "frames": entry["frames"] - 1,
            "facing": [round(c, 4) for c in to_gltf(rig.ahead)],
            "height": round(height, 4),
            "baseFeet": base_track,
            "feet": {
                side: corrected(base_track.get(side, track_of(solved["baseFeet"][side])), solved["baseFeet"][side], solved["feet"][side])
                for side in solved["feet"]
            },
            "stretch": solved["stretch"] if solved["stretch"]["mm"] is not None else None,
            "stretchNote": solved["stretchNote"],
            "reaches": solved["reaches"],
            "pins": solved["pins"],
            "travel": entry.get("travel", 0.0),
            "seconds": round(time.time() - started, 2),
        }
        with open(at(job["report"]), "w", encoding="utf-8") as f:
            json.dump(result, f, indent=2)
        progress.set("Done", 1, 1)
        log(f"wrote {job['out']}, {entry['frames'] - 1} frames as {name}")
    except Exception as err:
        traceback.print_exc()
        print(f"[recipe] Error: {err}", flush=True)
        raise


if __name__ == "__main__":
    main()
