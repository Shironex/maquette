"""surface.py: how well a posed skinned mesh holds its surface, for the pose test sheets.

Three numbers, in mm at a model one metre tall (its longest side for a creature):
  stretch_mm      the worst single edge: how much its length changed from rest. An edge pulled across a
                  bent joint changes length on any rig, so this alone does not say the weights are wrong.
  beyond_body_mm  how far an edge stretches past what a healthy skin does anyway, held over SURFACE_SHARE of the
                  surface: a handful of sliver edges in a dense mesh cannot set it, a tear that shows can. A healthy
                  skin is the kinder of two answers. The rigid body has every vertex stuck to its nearest bone: it answers for an edge
                  across a joint, but only there, since two ends on one bone give it nothing to say. The fair
                  skin keeps this model's own weights on nearby bones and evens them over a few rings: a blend
                  across a robe survives that, so it answers for the blend, while a hard step between two
                  neighbours, or weight on a bone far away, does not survive and stays counted. An edge may
                  also grow by half its own length: on a long edge of a coarse mesh that is mild, not a tear.
  worst_edge_mm   the single worst edge of that same reading, and worst_edge_near the bone nearest it: where to
                  look when a few edges pull hard but too little of the surface to call a tear.
  seam_gap_mm     how far vertices that share a position at rest (copies split at a UV seam) came apart:
                  a real crack. A split surface has no edge across it, so stretch never sees a tear.
"""
import numpy as np

# Rings of neighbours the fair skin evens weights over: flattens a step, leaves a blend of several rings.
FAIR_PASSES = 5
# A bone counts as near a vertex up to this share of the model past its nearest bone: a robe's hem may
# carry the hips, a hem on the arm may not.
FAIR_REACH = 0.35
# How much of its own length an edge may stretch by and still read as cloth.
EDGE_GIVE = 0.5
# How much of the surface has to stretch for the pose to read that far. On real robed models the worst edges are
# a rim of slivers between a layer on the leg and a layer on the trunk, far under this and mostly out of sight;
# torn cloth and a hem on the far arm cover well over it. Share of area, not of edges, so density does not matter.
SURFACE_SHARE = 0.005


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
        self.edge_area = self._edge_areas()
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
        dist = np.zeros((len(self.bones), len(self.rest)), np.float32)
        for i, (a, b) in enumerate(zip(heads, tails)):
            d = dist[i] = _segment_distance(self.rest, a, b)
            near = d < best
            best[near], self.nearest[near] = d[near], i
        self.rest_inv = [np.linalg.inv(_matrix(b.matrix_local)) for b in self.bones]
        self.fair = self._fair_weights(dist, best, inv)

    def _edge_areas(self):
        """Each edge's part of the surface: every face shared out over its edges."""
        m = self.me.data
        area, sides = np.zeros(len(m.polygons)), np.zeros(len(m.polygons), int)
        m.polygons.foreach_get("area", area)
        m.polygons.foreach_get("loop_total", sides)
        loop_edge = np.zeros(len(m.loops), int)
        m.loops.foreach_get("edge_index", loop_edge)
        out = np.zeros(len(m.edges))
        np.add.at(out, loop_edge, np.repeat(area / np.maximum(sides, 1), sides))
        return out

    def _over_share(self, beyond):
        """The stretch that SURFACE_SHARE of the surface reaches or passes."""
        total = float(self.edge_area.sum())
        if total <= 0:
            return float(beyond.max())
        order = np.argsort(-beyond)
        reached = np.searchsorted(np.cumsum(self.edge_area[order]), SURFACE_SHARE * total)
        return float(beyond[order[min(int(reached), len(order) - 1)]])

    def _fair_weights(self, dist, best, inv):
        """{bone index: (vertices, weights)} of the fair skin."""
        index = {b.name: i for i, b in enumerate(self.bones)}
        group = {g.index: index.get(g.name) for g in self.me.vertex_groups}
        found = [(v.index, group[g.group], g.weight) for v in self.me.data.vertices for g in v.groups if group.get(g.group) is not None]
        ids = sorted({f[1] for f in found} | set(np.unique(self.nearest).tolist()))
        col = {b: c for c, b in enumerate(ids)}
        w = np.zeros((len(self.rest), len(ids)), np.float32)
        for vert, bone, weight in found:
            if dist[bone, vert] <= best[vert] + FAIR_REACH * self.size:
                w[vert, col[bone]] = weight
        empty = w.sum(1) <= 0
        w[empty, np.array([col[b] for b in self.nearest[empty].tolist()], int)] = 1
        # Copies at a UV seam are one vertex here, or the skin would stop at every seam.
        count = inv.max() + 1 if len(inv) else 0
        each = np.bincount(inv, minlength=count)
        welded = np.stack([np.bincount(inv, w[:, c], count) / each for c in range(w.shape[1])], 1)
        a, b = inv[self.edges[:, 0]], inv[self.edges[:, 1]]
        keep = a != b
        a, b = a[keep], b[keep]
        degree = np.bincount(a, minlength=count) + np.bincount(b, minlength=count) + 1
        for c in range(welded.shape[1]):
            for _ in range(FAIR_PASSES):
                x = welded[:, c]
                welded[:, c] = (x + np.bincount(a, x[b], count) + np.bincount(b, x[a], count)) / degree
        welded /= welded.sum(1, keepdims=True)
        w = welded[inv]
        return {b: (np.flatnonzero(w[:, c] > 0), w[w[:, c] > 0, c]) for c, b in enumerate(ids)}

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

    def _fair_body(self):
        """The rest mesh under the fair skin, as posed now."""
        out = np.zeros_like(self.rest)
        for i, (at, w) in self.fair.items():
            skin = _matrix(self.arm.pose.bones[self.bones[i].name].matrix) @ self.rest_inv[i]
            out[at] += w[:, None] * _apply(self.from_arm @ skin @ self.to_arm, self.rest[at])
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
        rigid = np.abs(self._lengths(self._body()) - self.rest_len)
        fair = np.abs(self._lengths(self._fair_body()) - self.rest_len)
        beyond = np.maximum(change - np.maximum(np.maximum(rigid, fair), EDGE_GIVE * self.rest_len), 0)
        k = int(change.argmax()) if len(change) else 0
        b = int(beyond.argmax()) if len(beyond) else 0
        # For two copies, the box's diagonal is exactly how far apart they are.
        gap = max((float(np.linalg.norm(np.ptp(co[g], axis=0))) for g in self.seams), default=0.0)
        mm = lambda value: round(float(value) / size * 1000, 1)
        worst = self.edges[b if beyond[b] > 0 else k, 0] if len(change) else 0
        near = self.bones[self.nearest[worst]].name.split(":")[-1] if len(change) else ""
        return dict(stretch_mm=mm(change[k]) if len(change) else 0.0, beyond_body_mm=mm(self._over_share(beyond)) if len(beyond) else 0.0,
                    worst_edge_mm=mm(beyond[b]) if len(beyond) else 0.0, worst_edge_near=near,
                    seam_gap_mm=mm(gap), at=[round(float(x), 3) for x in self.rest[worst]] if len(change) else [0, 0, 0])
