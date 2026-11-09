"""Holds a recipe's pins, last in each frame, after the clips are joined, the
base edited and every other change laid over. bake_recipe.py imports it; it
does nothing alone.

The job's recipe carries each pin on the bake's frames as Maquette worked it
out (the core's recipes/pins.rs, which the rough preview reads too):

    "pins": [{"layer": "l-1", "bone": "leftFoot", "first": 12, "last": 80,
              "easeIn": 3, "easeOut": 3, "weight": 1.0,
              "flat": "toe-and-heel", "target": null}]

At its first frame a pin takes its spot: where the hand or foot is then, as
everything before it left it, or the blank's target when it names one. From
there an IK constraint holds the bone's head on the spot, up through its upper
arm or leg, its influence eased in and out over the pin's frames as the core
says. A limb held dead straight is bent a few degrees first, as a reach's is,
so the solver knows which way to fold it.

What of the bone keeps its turn: a hand keeps the turn it had at its first
frame; a foot keeps it whole for "toe-and-heel", only its heading (the way the
toes point, level) for "heel", so the toe may roll, and none for "off". A
Copy Rotation constraint on an empty at the spot holds it.

While a pin holds whole (past its ease in, before its ease out) each frame
measures how far the bone's head is from the spot (the drift), and how far it
was before the pin held it (what it would have moved without the pin).
"""

import math

import bpy
from mathutils import Quaternion, Vector

UP = Vector((0.0, 0.0, 1.0))
# A limb is held straight within this angle, and is bent this far before
# its pin is solved.
STRAIGHT = math.radians(3.0)
PREBEND = math.radians(10.0)
# A pin's end bone, with the upper and lower bones its IK solves.
LIMBS = {
    "leftHand": ("leftUpperArm", "leftLowerArm"),
    "rightHand": ("rightUpperArm", "rightLowerArm"),
    "leftFoot": ("leftUpperLeg", "leftLowerLeg"),
    "rightFoot": ("rightUpperLeg", "rightLowerLeg"),
}


def smooth(x):
    if x <= 0.0:
        return 0.0
    if x >= 1.0:
        return 1.0
    return x * x * (3.0 - 2.0 * x)


def amount(spec, frame):
    """How much of a pin holds at bake frame `frame`, 0 to 1, weight in."""
    first, last = int(spec["first"]), int(spec["last"])
    if frame < first or frame > last:
        return 0.0
    ease_in, ease_out = int(spec.get("easeIn", 0)), int(spec.get("easeOut", 0))
    up = smooth((frame - first) / ease_in) if ease_in > 0 else 1.0
    down = smooth((last - frame) / ease_out) if ease_out > 0 else 1.0
    return float(spec.get("weight", 1.0)) * min(up, down)


def chain_length(arm, end, upper):
    """How many bones an IK on `end` (its head) solves to reach up to `upper`."""
    count, bone = 0, arm.data.bones[end].parent
    while bone is not None:
        count += 1
        if bone.name == upper:
            return count
        bone = bone.parent
    raise RuntimeError(f"{end} does not hang under {upper}, so its pin cannot be solved")


def level(v):
    return Vector((v.x, v.y, 0.0))


class Pin:
    """One pin: its constraints on an empty at its spot, and what is measured."""

    def __init__(self, rig, spec, target, scene):
        self.spec = spec
        self.first = int(spec["first"])
        self.held = (self.first + int(spec.get("easeIn", 0)), int(spec["last"]) - int(spec.get("easeOut", 0)))
        end_vrm = spec["bone"]
        upper_vrm, lower_vrm = LIMBS[end_vrm]
        self.end, self.upper, self.lower = (rig.pose_bone(v) for v in (end_vrm, upper_vrm, lower_vrm))
        if not (self.end and self.upper and self.lower):
            raise RuntimeError(f"the model has no {end_vrm} limb to pin")
        self.target = target
        self.foot = end_vrm.endswith("Foot")
        self.flat = spec.get("flat") or ("toe-and-heel" if self.foot else None)
        self.empty = bpy.data.objects.new(f"recipe_pin_{spec['layer']}", None)
        scene.collection.objects.link(self.empty)
        self.empty.rotation_mode = "QUATERNION"
        self.ik = self.end.constraints.new("IK")
        self.ik.name = f"recipe_pin_{spec['layer']}"
        self.ik.target = self.empty
        self.ik.use_tail = False
        self.ik.chain_count = chain_length(rig.arm, self.end.name, self.upper.name)
        self.ik.influence = 0.0
        self.keep = None
        if self.flat != "off":
            self.keep = self.end.constraints.new("COPY_ROTATION")
            self.keep.name = f"recipe_pin_turn_{spec['layer']}"
            self.keep.target = self.empty
            self.keep.influence = 0.0
        self.spot = None
        self.turn = None
        self.before = None
        self.drift = None
        self.drift_frame = None
        self.base_drift = None

    def reset(self):
        self.ik.influence = 0.0
        if self.keep is not None:
            self.keep.influence = 0.0

    def prebend(self, rig, influence):
        root, mid, end = rig.head(self.upper), rig.head(self.lower), rig.head(self.end)
        upper, lower = mid - root, end - mid
        if upper.length < 1e-6 or lower.length < 1e-6:
            return
        if upper.normalized().cross(lower.normalized()).length > math.sin(STRAIGHT):
            return
        way = -rig.ahead if self.foot else rig.ahead
        axis = lower.cross(way)
        if axis.length < 1e-6:
            axis = lower.cross(rig.left)
        rig.turn(self.lower, Quaternion(axis.normalized(), PREBEND * influence))

    def heading_kept(self, rig):
        """The bone's turn now, turned about the upright so its toes point the
        way they did at the first frame: its heading kept, the roll free."""
        now = rig.world_rotation(self.end)
        axis = Vector((0.0, 1.0, 0.0))
        want, have = level(self.turn @ axis), level(now @ axis)
        if want.length < 1e-6 or have.length < 1e-6:
            return now
        angle = math.atan2(have.cross(want).dot(UP), have.dot(want))
        return Quaternion(UP, angle) @ now

    def hold(self, rig, frame):
        """Takes the spot at the first frame, then holds the bone on it."""
        if frame == self.first:
            self.spot = (self.target if self.target is not None else rig.head(self.end)).copy()
            self.turn = rig.world_rotation(self.end)
            self.empty.location = self.spot
            self.empty.rotation_quaternion = self.turn
        if self.spot is None:
            return
        self.before = rig.head(self.end).copy()
        share = min(amount(self.spec, frame), 1.0)
        if share <= 0.0:
            return
        self.ik.influence = share
        self.prebend(rig, share)
        bpy.context.view_layer.update()
        if self.keep is None:
            return
        if self.flat == "heel":
            self.empty.rotation_quaternion = self.heading_kept(rig)
        else:
            self.empty.rotation_quaternion = self.turn
        self.keep.influence = share
        bpy.context.view_layer.update()

    def measure(self, rig, frame):
        if self.spot is None or not (self.held[0] <= frame <= self.held[1]):
            return
        drift = (rig.head(self.end) - self.spot).length
        if self.drift is None or drift > self.drift:
            self.drift, self.drift_frame = drift, frame
        base = (self.before - self.spot).length if self.before is not None else 0.0
        self.base_drift = base if self.base_drift is None else max(self.base_drift, base)

    def remove(self):
        self.end.constraints.remove(self.ik)
        if self.keep is not None:
            self.end.constraints.remove(self.keep)
        bpy.data.objects.remove(self.empty)

    def report(self):
        return {
            "layer": self.spec["layer"],
            "bone": self.spec["bone"],
            "first": self.first,
            "last": int(self.spec["last"]),
            "drift": self.drift,
            "frame": self.drift_frame,
            "baseDrift": self.base_drift,
        }


class Pins:
    """Every pin of a recipe, laid in order."""

    def __init__(self, rig, specs, targets, scene):
        self.pins = []
        for spec in specs:
            target = None
            if spec.get("target"):
                target = targets.get(spec["target"])
                if target is None:
                    raise RuntimeError(f"the pin {spec['layer']} has no target")
            self.pins.append(Pin(rig, spec, target, scene))

    def reset(self):
        for pin in self.pins:
            pin.reset()

    def hold(self, rig, frame):
        for pin in self.pins:
            pin.hold(rig, frame)

    def measure(self, rig, frame):
        for pin in self.pins:
            pin.measure(rig, frame)

    def remove(self):
        for pin in self.pins:
            pin.remove()

    def report(self):
        return [pin.report() for pin in self.pins]
