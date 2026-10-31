"""narrow.py: the narrow reweight for a mesh that is one continuous shell: tail and hair off the arms.

fixweights.py decides per connected island, so on one island it would give the whole body to a limb or
cut sleeves and legs apart (it refuses). This script changes far less: only arm and hand weight on
vertices well outside the arm goes, into trunk weights, faded in with distance so there is no hard cut.
Legs, sleeves, hands and everything near an arm keep their weights. Only JOINTS_0 and WEIGHTS_0 change.

Usage: blender -b --factory-startup -P narrow.py -- in.glb out.glb
       (or Blender's own python: <python> -I narrow.py in.glb out.glb)
Writes out.glb.report.json with what moved.
"""
import sys, os, json
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np, glbio

NEAR, FAR = 0.07, 0.13   # of the model's height: arm weight stays inside NEAR and is gone beyond FAR
P_IDW = 4.0


def seg_dist(P, a, b):
    ab = b - a
    t = np.clip(((P - a) @ ab) / max(ab @ ab, 1e-12), 0, 1)
    return np.linalg.norm(P - (a + t[:, None] * ab), axis=1)


def smoothstep(x):
    x = np.clip(x, 0, 1)
    return x * x * (3 - 2 * x)


def main(src, dst):
    g = glbio.load(src)
    H = glbio.height(g)
    PFX = next(n for n in g.names if n.endswith("Hips"))[:-4]
    ni = {n: i for i, n in enumerate(g.names)}
    JP = lambda n: g.jpos[ni[PFX + n]]
    W0 = glbio.dense(g)
    W = W0.copy()
    trunk = [(a, b) for a, b in (("Hips", "Spine"), ("Spine", "Spine1"), ("Spine1", "Spine2"), ("Spine2", "Neck"), ("Neck", "Head")) if PFX + a in ni and PFX + b in ni]
    dd = np.stack([seg_dist(g.pos, JP(a), JP(b)) for a, b in trunk], 1)
    iw = 1.0 / np.maximum(dd, 0.01 * H) ** P_IDW
    Wt = np.zeros_like(W0)
    for col, (a, _) in enumerate(trunk):
        Wt[:, ni[PFX + a]] += iw[:, col]
    Wt /= Wt.sum(1, keepdims=True)
    moved = 0
    for side in ("Left", "Right"):
        cols = [i for i, n in enumerate(g.names) if n.startswith(PFX + side) and any(n[len(PFX + side):].startswith(k) for k in ("Arm", "ForeArm", "Hand"))]
        chain = [n for n in ("Arm", "ForeArm", "Hand") if PFX + side + n in ni]
        segs = [(JP(side + a), JP(side + b)) for a, b in zip(chain, chain[1:])]
        fingers = [i for i, n in enumerate(g.names) if n.startswith(PFX + side + "Hand") and n != PFX + side + "Hand"]
        segs += [(JP(side + "Hand"), g.jpos[i]) for i in fingers]
        d = np.min([seg_dist(g.pos, a, b) for a, b in segs], axis=0)
        # hand and finger weight answers to the forearm and hand alone, so a strand beside the shoulder loses it
        hand_cols = [i for i in cols if g.names[i].startswith(PFX + side + "Hand")]
        d_hand = np.min([seg_dist(g.pos, a, b) for a, b in segs[1:]], axis=0)
        lost = np.zeros(len(g.pos))
        for col in cols:
            dist = d_hand if col in hand_cols else d
            taken = W[:, col] * smoothstep((dist - NEAR * H) / ((FAR - NEAR) * H))
            W[:, col] -= taken
            lost += taken
        W += lost[:, None] * Wt
        moved += int((lost > 0.01).sum())
    joints, weights = glbio.to_top4(W)
    glbio.save_weights(g, joints, weights, dst)
    Wn = np.zeros_like(W0); np.add.at(Wn, (np.repeat(np.arange(len(g.pos)), 4), joints.astype(int).ravel()), weights.ravel())
    report = dict(narrow=True, verts=len(g.pos), tris=len(g.idx), moved=moved, changed=int((np.abs(Wn - W0).sum(1) > 0.05).sum()), limbs={})
    print("REPORT", json.dumps(report))
    json.dump(report, open(dst + ".report.json", "w"), indent=1)


if __name__ == "__main__":
    a = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:]
    main(a[0], a[1])
