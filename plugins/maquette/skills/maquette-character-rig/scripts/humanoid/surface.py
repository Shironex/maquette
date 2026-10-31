"""surface.py: how well a posed skinned mesh holds its surface, for the pose test sheets.

Three numbers, in mm at a model one metre tall (its longest side for a creature):
  stretch_mm      the worst single edge: how much its length changed from rest. An edge pulled across a
                  bent joint changes length on any rig, so this alone does not say the weights are wrong.
  beyond_body_mm  the same worst edge after taking off what the body does anyway. The body here is the
                  mesh with every vertex stuck to its nearest bone, whatever its weights say: what that
                  rigid body does to an edge is the pose, and only the change beyond it is the weights.
  seam_gap_mm     how far vertices that share a position at rest (copies split at a UV seam) came apart:
                  a real crack. A split surface has no edge across it, so stretch never sees a tear.
"""
import numpy as np


def _matrix(m):
    return np.array([list(row) for row in m], float)


def _apply(m, co):
    return co @ m[:3, :3].T + m[:3, 3]


def _segment_distance(p, a, b):
    ab = b - a
    t = np.clip(((p - a) @ ab) / max(float(ab @ ab), 1e-12), 0, 1)
    return np.linalg.norm(p - (a + t[:, None] * ab), axis=1)


class Surface:
    """Reads the rest mesh once. Build it before posing anything."""

    def __init__(self, arm, me):
        self.arm, self.me = arm, me
        m = me.data
        co = np.zeros(len(m.vertices) * 3)
        m.vertices.foreach_get("co", co)
        self.rest = co.reshape(-1, 3)
        ed = np.zeros(len(m.edges) * 2, int)
        m.edges.foreach_get("vertices", ed)
        self.edges = ed.reshape(-1, 2)
        self.rest_len = self._lengths(self.rest)
        # The model's own size in the units of its vertices: a pose or a parent's scale cannot change it.
        self.size = max(float(np.ptp(self.rest, axis=0).max()), 1e-6)
        self.height = max(float(np.ptp(self.rest[:, 2])), 1e-6)
        _, inv = np.unique(np.round(self.rest, 5), axis=0, return_inverse=True)
        inv = inv.ravel()
        order = np.argsort(inv)
        self.seams = [g for g in np.split(order, np.flatnonzero(np.diff(inv[order])) + 1) if len(g) > 1]
        self.to_arm = _matrix(arm.matrix_world.inverted() @ me.matrix_world)
        self.from_arm = np.linalg.inv(self.to_arm)
        self.bones = list(arm.data.bones)
        heads = _apply(self.from_arm, np.array([list(b.head_local) for b in self.bones], float))
        tails = _apply(self.from_arm, np.array([list(b.tail_local) for b in self.bones], float))
        best, self.nearest = np.full(len(self.rest), np.inf), np.zeros(len(self.rest), int)
        for i, (a, b) in enumerate(zip(heads, tails)):
            d = _segment_distance(self.rest, a, b)
            near = d < best
            best[near], self.nearest[near] = d[near], i
        self.rest_inv = [np.linalg.inv(_matrix(b.matrix_local)) for b in self.bones]

    def _lengths(self, co):
        return np.linalg.norm(co[self.edges[:, 0]] - co[self.edges[:, 1]], axis=1)

    def _body(self):
        """The rest mesh with every vertex stuck to its nearest bone, as posed now."""
        out = self.rest.copy()
        for i in np.unique(self.nearest):
            skin = _matrix(self.arm.pose.bones[self.bones[i].name].matrix) @ self.rest_inv[i]
            at = self.nearest == i
            out[at] = _apply(self.from_arm @ skin @ self.to_arm, self.rest[at])
        return out

    def posed(self):
        import bpy
        ev = self.me.evaluated_get(bpy.context.evaluated_depsgraph_get()).data
        co = np.zeros(len(ev.vertices) * 3)
        ev.vertices.foreach_get("co", co)
        return co.reshape(-1, 3)

    def measure(self, size=None):
        """The pose's numbers. `size` is what counts as one metre: the model's height by default."""
        size = size or self.height
        co = self.posed()
        change = np.abs(self._lengths(co) - self.rest_len)
        beyond = np.maximum(change - np.abs(self._lengths(self._body()) - self.rest_len), 0)
        k = int(change.argmax()) if len(change) else 0
        b = int(beyond.argmax()) if len(beyond) else 0
        # For two copies, the box's diagonal is exactly how far apart they are.
        gap = max((float(np.linalg.norm(np.ptp(co[g], axis=0))) for g in self.seams), default=0.0)
        mm = lambda value: round(float(value) / size * 1000, 1)
        at = self.rest[self.edges[b if beyond[b] > 0 else k, 0]] if len(change) else (0, 0, 0)
        return dict(stretch_mm=mm(change[k]) if len(change) else 0.0, beyond_body_mm=mm(beyond[b]) if len(beyond) else 0.0,
                    seam_gap_mm=mm(gap), at=[round(float(x), 3) for x in at])
