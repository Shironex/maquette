"""rigid.py: binds parts rigidly to one body bone, so a wing or a cape weighted to the arms follows the torso.

The parts are what the arm chains hold away from the arm: each connected patch of vertices that an arm chain
(upper arm, forearm, hand, fingers) holds at least a quarter of, yet that sits outside the arm's tube. A wing,
a cape or a braid on the arm bones each make one. They are read from the source's weights, so the list stays
the same after the fix moved weight. Binding gives the part's vertices the one bone, and blends it into
the rings of vertices around the part, so nothing tears where the part meets the rest. Only JOINTS_0 and
WEIGHTS_0 change.

Usage: blender -b --factory-startup -P rigid.py -- src.glb --parts out.json
           lists the parts (largest first, numbered from 1) and the body bones a part can follow
       blender -b --factory-startup -P rigid.py -- src.glb base.glb out.glb --part N[,N...] --bone NAME
           binds those parts of src.glb onto base.glb's weights (the fix's draft) and writes out.glb
       (or Blender's own python: <python> -I rigid.py ...; it needs numpy only)
Writes out.glb.report.json with what moved.
"""
import sys, os, json
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np, glbio

TUBE = 0.045         # of the model's height: the arm's own surface lies inside this from its bones
HELD = 0.25          # an arm chain holds a vertex when it has at least this share of its weight
MIN_PART = 16        # vertices, and at least MIN_SHARE of the mesh, for a patch to count as a part
MIN_SHARE = 0.002
MAX_PARTS = 8
RINGS = 3            # rings of neighbours the bone blends into, fading out
BODY = ("Spine2", "Spine1", "Spine", "Hips", "Neck", "Head", "LeftShoulder", "RightShoulder")


def seg_dist(P, a, b):
    ab = b - a
    t = np.clip(((P - a) @ ab) / max(ab @ ab, 1e-12), 0, 1)
    return np.linalg.norm(P - (a + t[:, None] * ab), axis=1)


def prefix(g):
    return next(n for n in g.names if n.endswith("Hips"))[:-4]


def neighbours(g, inv):
    """Each welded vertex's neighbours over the triangles."""
    t = inv[g.idx]
    pairs = np.concatenate([t[:, [0, 1]], t[:, [1, 2]], t[:, [2, 0]]])
    n = inv.max() + 1
    out = [[] for _ in range(n)]
    for a, b in pairs:
        if a != b:
            out[a].append(b); out[b].append(a)
    return out


def read_parts(g):
    """The parts, largest first, each as (welded vertex ids, side, the arm bones holding it)."""
    H = glbio.height(g)
    PFX = prefix(g)
    ni = {n: i for i, n in enumerate(g.names)}
    W = glbio.dense(g)
    _, inv = glbio.components(g)
    nb = neighbours(g, inv)
    nw = inv.max() + 1
    off = np.zeros(nw, bool)
    side_of = {}
    for side in ("Left", "Right"):
        chain = [n for n in ("Arm", "ForeArm", "Hand") if PFX + side + n in ni]
        if len(chain) < 2:
            continue
        cols = [i for i, n in enumerate(g.names) if n.startswith(PFX + side) and any(n[len(PFX + side):].startswith(k) for k in ("Arm", "ForeArm", "Hand"))]
        jp = lambda n: g.jpos[ni[PFX + side + n]]
        segs = [(jp(a), jp(b)) for a, b in zip(chain, chain[1:])]
        segs += [(jp("Hand"), g.jpos[i]) for i in cols if g.names[i].startswith(PFX + side + "Hand") and g.names[i] != PFX + side + "Hand"]
        d = np.min([seg_dist(g.pos, a, b) for a, b in segs], axis=0)
        mine = (W[:, cols].sum(1) >= HELD) & (d > TUBE * H)
        for v in np.nonzero(mine)[0]:
            off[inv[v]] = True
            side_of.setdefault(inv[v], side)
    seen = np.zeros(nw, bool)
    parts = []
    for start in np.nonzero(off)[0]:
        if seen[start]:
            continue
        stack, patch = [start], []
        seen[start] = True
        while stack:
            v = stack.pop(); patch.append(v)
            for u in nb[v]:
                if off[u] and not seen[u]:
                    seen[u] = True; stack.append(u)
        parts.append(patch)
    floor = max(MIN_PART, MIN_SHARE * nw)
    parts = sorted((p for p in parts if len(p) >= floor), key=len, reverse=True)[:MAX_PARTS]
    out = []
    for patch in parts:
        verts = np.nonzero(np.isin(inv, patch))[0]
        sides = {side_of[v] for v in patch}
        totals = W[verts].sum(0)
        held = [g.names[i][len(PFX):] for i in np.argsort(-totals)[:3] if totals[i] > 0 and g.names[i][len(PFX):].startswith(("Left", "Right"))]
        out.append((patch, "both" if len(sides) > 1 else sides.pop().lower(), held, verts))
    return out, inv, nb, H


def listing(g):
    parts, _, _, H = read_parts(g)
    PFX = prefix(g)
    rows = []
    for i, (patch, side, held, verts) in enumerate(parts, 1):
        p = g.pos[verts]
        across = float((p.max(0) - p.min(0)).max() / max(H, 1e-9) * 100)
        rows.append(dict(part=i, vertices=int(len(verts)), side=side, bones=held, across_cm=round(across, 1)))
    bones = [PFX + b for b in BODY if PFX + b in g.names]
    return dict(parts=rows, bones=bones, height=H)


def bind(src, base, dst, chosen, bone):
    g = glbio.load(src)
    parts, inv, nb, _ = read_parts(g)
    for part in chosen:
        if not 1 <= part <= len(parts):
            raise SystemExit(f"rigid.py: there is no part {part}; the model has {len(parts)}")
    if bone not in g.names:
        raise SystemExit(f"rigid.py: the model has no bone {bone}")
    b = glbio.load(base)
    W0 = glbio.dense(b)
    W = W0.copy()
    one = np.zeros(len(g.names)); one[g.names.index(bone)] = 1.0
    share = np.zeros(inv.max() + 1)
    ring = [v for part in chosen for v in parts[part - 1][0]]
    share[ring] = 1.0
    for k in range(1, RINGS + 1):
        nxt = {u for v in ring for u in nb[v] if share[u] == 0}
        ring = list(nxt)
        share[ring] = 1.0 - k / (RINGS + 1)
    a = share[inv][:, None]
    W = (1 - a) * W + a * one
    joints, weights = glbio.to_top4(W)
    glbio.save_weights(b, joints, weights, dst)
    Wn = np.zeros_like(W0); np.add.at(Wn, (np.repeat(np.arange(len(b.pos)), 4), joints.astype(int).ravel()), weights.ravel())
    report = dict(parts=chosen, bone=bone, vertices=int(sum(len(parts[p - 1][3]) for p in chosen)), band=int(((share[inv] > 0) & (share[inv] < 1)).sum()),
                  changed=int((np.abs(Wn - W0).sum(1) > 0.05).sum()))
    print("REPORT", json.dumps(report))
    json.dump(report, open(dst + ".report.json", "w"), indent=1)


if __name__ == "__main__":
    a = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:]
    if "--parts" in a:
        found = listing(glbio.load(a[0]))
        print("PARTS", json.dumps(found["parts"]))
        json.dump(found, open(a[a.index("--parts") + 1], "w"), indent=1)
    else:
        chosen = sorted({int(p) for p in a[a.index("--part") + 1].split(",")})
        bind(a[0], a[1], a[2], chosen, a[a.index("--bone") + 1])
