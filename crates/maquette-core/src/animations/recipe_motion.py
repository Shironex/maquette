"""Builds a recipe's motion from its clips, frame by frame, before its
changes are laid over it. bake_recipe.py imports it; it does nothing alone.

The job's recipe carries the timeline Maquette worked out (the core's
recipes/timeline.rs, which the rough preview plays too): for every frame of
the new clip, the frame of the clip it shows ("from") and, in a blend window,
the next clip's frame ("to") with how much of it shows ("blend"); a freeze or
hold over showing there ("hold": its frame, its weight and its breathe); and
the seams between clips with what each lines up. Each clip was put on the
model whole by the retarget, as an action of its own ("parts").

Per frame:

1. A clip's pose at its frame is read from its action's keys (between keys
   when a retime steps by part of a frame). A hold turns it toward the pose
   of the held frame, by the hold's weight.
2. A joined clip is lined up with the clip before it at its seam: moved level
   so its hips sit over the hips of the clip before ("hips"), and turned about
   the upright through its hips so they face the same way too
   ("hips-and-facing"). That move applies to all its frames, and the clips
   after it line up with it as lined up.
3. In a blend window each bone turns from one clip's pose to the next's (the
   hips also move), eased as the timeline's blend says.
4. Mirror swaps left and right: each bone takes its partner's pose reflected
   across the model's middle (the plane through its origin across its left
   axis), so a right handed slash becomes a left handed one.

A held pose breathes: bake_recipe.py tips the chest (and half as much the
spine) about the model's left axis, BREATHE_DEGREES at full breathe, once
every BREATHE_SECONDS.
"""

import math
import re

from mathutils import Matrix, Quaternion, Vector

BREATHE_DEGREES = 3.0
BREATHE_SECONDS = 3.0
STILL = (1.0, 0.0, 0.0, 0.0)
# Names a bone may carry its side in, left then right.
SIDES = (
    (r"\.L$", ".R"), (r"\.R$", ".L"), (r"_L$", "_R"), (r"_R$", "_L"),
    (r"\.l$", ".r"), (r"\.r$", ".l"), (r"_l$", "_r"), (r"_r$", "_l"),
    (r"Left", "Right"), (r"Right", "Left"), (r"left", "right"), (r"right", "left"),
)


class Part:
    """One clip's keys on the model: each bone's turn and move at a frame of
    the clip. A part with no action holds the rest pose."""

    def __init__(self, action, step, bones):
        self.step = step
        curves = {}
        if action is not None:
            for layer in action.layers:
                for strip in layer.strips:
                    for bag in strip.channelbags:
                        for fc in bag.fcurves:
                            curves[(fc.data_path, fc.array_index)] = fc
        self.rot = {}
        self.loc = {}
        for name in bones:
            path = f'pose.bones["{name}"]'
            self.rot[name] = [curves.get((f"{path}.rotation_quaternion", i)) for i in range(4)]
            found = [curves.get((f"{path}.location", i)) for i in range(3)]
            if any(found):
                self.loc[name] = found

    def pose(self, at):
        frame = at * self.step
        out = {}
        for name, curves in self.rot.items():
            q = Quaternion([c.evaluate(frame) if c else d for c, d in zip(curves, STILL)])
            q.normalize()
            loc = self.loc.get(name)
            v = Vector([c.evaluate(frame) if c else 0.0 for c in loc]) if loc else Vector()
            out[name] = (q, v)
        return out


def mix(a, b, w):
    """Pose `a` turned `w` of the way to pose `b`, bone by bone."""
    if w <= 0.0:
        return a
    if w >= 1.0:
        return b
    out = {}
    for name, (qa, la) in a.items():
        qb, lb = b[name]
        if qa.dot(qb) < 0.0:
            qb = -qb
        out[name] = (qa.slerp(qb, w), la.lerp(lb, w))
    return out


class Skeleton:
    """The model's bones parents first, their rest matrices, and its upright,
    left and ahead in the armature's own space."""

    def __init__(self, arm, order, hips, rig):
        self.order = [b.name for b in order]
        self.rest = {b.name: b.matrix_local.copy() for b in order}
        self.parent = {b.name: b.parent.name if b.parent else None for b in order}
        self.hips = hips
        inv = rig.mw_rot_inv
        self.up = (inv @ Vector((0.0, 0.0, 1.0))).normalized()
        self.left = (inv @ rig.left).normalized()
        self.ahead = (inv @ rig.ahead).normalized()

    def base(self, posed, name):
        parent = self.parent[name]
        if parent is None:
            return self.rest[name]
        return posed[parent] @ self.rest[parent].inverted() @ self.rest[name]

    def armature(self, pose):
        """Each bone's matrix in armature space for `pose`."""
        posed = {}
        for name in self.order:
            q, v = pose[name]
            posed[name] = self.base(posed, name) @ Matrix.Translation(v) @ q.to_matrix().to_4x4()
        return posed

    def local(self, posed, moved):
        """`posed` back as each bone's own turn, and move for the bones in `moved`."""
        out = {}
        for name in self.order:
            basis = self.base(posed, name).inverted() @ posed[name]
            loc, q, _ = basis.decompose()
            out[name] = (q.normalized(), loc if name in moved else Vector())
        return out

    def level(self, v):
        return v - self.up * v.dot(self.up)

    def hips_matrix(self, pose):
        """The hips in armature space, the bones above them at rest."""
        posed = {}
        name = self.hips
        chain = []
        while name is not None:
            chain.append(name)
            name = self.parent[name]
        for n in reversed(chain):
            q, v = pose[n] if n == self.hips else (Quaternion(), Vector())
            posed[n] = self.base(posed, n) @ Matrix.Translation(v) @ q.to_matrix().to_4x4()
        return posed[self.hips]

    def facing(self, hips):
        turn = hips.to_3x3() @ self.rest[self.hips].to_3x3().inverted()
        return self.level(turn @ self.ahead)

    def line_up(self, before, after, how):
        """The move (armature space) that lines pose `after` up with `before`."""
        if how == "off":
            return Matrix()
        a, b = self.hips_matrix(before), self.hips_matrix(after)
        pa, pb = a.translation, b.translation
        angle = 0.0
        fa, fb = self.facing(a), self.facing(b)
        if how == "hips-and-facing" and fa.length > 1e-6 and fb.length > 1e-6:
            angle = math.atan2(fb.cross(fa).dot(self.up), fb.dot(fa))
        turn = Matrix.Rotation(angle, 4, self.up)
        return Matrix.Translation(pb + self.level(pa - pb)) @ turn @ Matrix.Translation(-pb)

    def moved(self, pose, move):
        """`pose` with its hips moved by `move` (armature space)."""
        if move is None:
            return pose
        q, v = pose[self.hips]
        base = self.hips_matrix({self.hips: (Quaternion(), Vector())})
        m = base.inverted() @ move @ base @ Matrix.Translation(v) @ q.to_matrix().to_4x4()
        loc, rot, _ = m.decompose()
        out = dict(pose)
        out[self.hips] = (rot.normalized(), loc)
        return out


def partners(names, pairs):
    """Each bone's left or right partner: the bone map's pairs, else the bone
    whose name differs only in its side; a bone with neither is its own."""
    out = {name: name for name in names}
    for left, right in pairs:
        if left in out and right in out:
            out[left], out[right] = right, left
    known = {n for pair in pairs for n in pair}
    for name in names:
        if name in known:
            continue
        for pattern, other in SIDES:
            swapped = re.sub(pattern, other, name, count=1)
            if swapped != name and swapped in out:
                out[name] = swapped
                break
    return out


def mirrored(sk, posed, partner):
    """Every bone takes its partner's armature pose reflected across the
    model's middle: S M_p rest_p^-1 S rest_b, S the reflection."""
    n = sk.left
    s = Matrix.Identity(4)
    for r in range(3):
        for c in range(3):
            s[r][c] -= 2.0 * n[r] * n[c]
    out = {}
    for name in sk.order:
        p = partner[name]
        out[name] = s @ posed[p] @ sk.rest[p].inverted() @ s @ sk.rest[name]
    return out


def breathing(key, t, fps):
    """How far a held pose tips at timeline frame `t`, radians."""
    amount = 0.0
    for sample, share in ((key["from"], 1.0 - key["blend"]), (key.get("to"), key["blend"])):
        hold = sample.get("hold") if sample else None
        if hold:
            amount += share * hold["weight"] * hold["breathe"]
    if amount <= 0.0:
        return 0.0
    cycle = 2.0 * math.pi * t / (fps * BREATHE_SECONDS)
    return math.radians(BREATHE_DEGREES) * amount * math.sin(cycle)


def build(sk, parts, timeline, pairs):
    """Every frame's pose: each bone's turn and move, plus how far each frame
    breathes. `parts[i]` is clip i's Part."""
    moved = {n for part in parts for n in part.loc}
    cache = {}

    def raw(sample):
        key = (sample["clip"], round(sample["at"], 6))
        if key not in cache:
            cache[key] = parts[sample["clip"]].pose(sample["at"])
        pose = cache[key]
        hold = sample.get("hold")
        if hold:
            held = (sample["clip"], float(hold["at"]))
            if held not in cache:
                cache[held] = parts[sample["clip"]].pose(float(hold["at"]))
            pose = mix(pose, cache[held], hold["weight"])
        return pose

    moves = {0: None}
    for seam in timeline["seams"]:
        clip = seam["clip"]
        before = sk.moved(raw(seam["from"]), moves.get(clip - 1))
        # The clip before is compared as lined up, so the move is whole.
        moves[clip] = sk.line_up(before, raw(seam["to"]), seam["lineUp"])
    partner = partners(sk.order, pairs) if timeline.get("mirror") else None
    fps = float(timeline["fps"])
    poses, breaths = [], []
    for t, key in enumerate(timeline["keys"]):
        a = sk.moved(raw(key["from"]), moves.get(key["from"]["clip"]))
        to = key.get("to")
        pose = mix(a, sk.moved(raw(to), moves.get(to["clip"])), key["blend"]) if to else a
        if partner is not None:
            pose = sk.local(mirrored(sk, sk.armature(pose), partner), moved)
        poses.append(pose)
        breaths.append(breathing(key, t, fps))
    return poses, breaths, moved
