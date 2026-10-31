"""probe.py: count vertices far from a limb that still carry that limb's weight, and read the islands.

Usage: blender -b --factory-startup -P probe.py -- file.glb [report.json] [--json out.json]
For each side, vertices with hand-chain weight (Hand + fingers) above 0.05 and farther than 15 percent
of the model's height from the Hand joint are 'far'. Vertices with arm weight (Arm, ForeArm) above 0.05
and farther than 10 percent of the height from the arm chain are 'arm far': hair and capes hang on arm
bones as often as on fingers. If a fixweights report says a limb's chain is misplaced, the hand distance
is also measured from where the mesh's wrist really is (the registered Hand joint), which is the
meaningful number. It also says how many connected islands the mesh has: one big island means fixweights
must not run (see glbio.one_island).
"""
import sys, os, json
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np, glbio
HAND_FAR, ARM_FAR, MIN_WEIGHT = 0.15, 0.10, 0.05
ARGS = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:]
json_out = ARGS[ARGS.index("--json") + 1] if "--json" in ARGS else None
ARGS = [a for i, a in enumerate(ARGS) if a != "--json" and (i == 0 or ARGS[i - 1] != "--json")]
g = glbio.load(ARGS[0]); W = glbio.dense(g); lab, _ = glbio.components(g)
rep = json.load(open(ARGS[1])) if len(ARGS) > 1 else {"limbs": {}}
PFX = next(n for n in g.names if n.endswith("Hips"))[:-4]
H = glbio.height(g)
ni = {n: i for i, n in enumerate(g.names)}
neck_y = g.jpos[ni[PFX + "Neck"]][1] if PFX + "Neck" in ni else g.pos[:, 1].max()


def seg_dist(P, a, b):
    ab = b - a
    t = np.clip(((P - a) @ ab) / max(ab @ ab, 1e-12), 0, 1)
    return np.linalg.norm(P - (a + t[:, None] * ab), axis=1)


def regions(mask):
    """Hair sits above the neck or hangs from it behind the head; the rest of a stray set is cloth."""
    high = g.pos[mask][:, 1] > neck_y - 0.12 * H
    return {"hair": int(high.sum()), "cloth": int((~high).sum())}


out = {}
bones = {}
places = {"hand": {"hair": 0, "cloth": 0}, "arm": {"hair": 0, "cloth": 0}}
for side in ("Left", "Right"):
    hc = [i for i, n in enumerate(g.names) if n.startswith(PFX + side + "Hand")]
    m = W[:, hc].sum(1)
    hj = g.jpos[ni[PFX + side + "Hand"]]
    refs = {"hand_joint": hj}
    rj = rep["limbs"].get(side + "arm", {}).get("registered_joints")
    if rj:
        refs["mesh_wrist"] = np.array(rj[PFX + side + "Hand"])
    for k, ref in refs.items():
        far = (m > MIN_WEIGHT) & (np.linalg.norm(g.pos - ref, axis=1) > HAND_FAR * H)
        out[f"{side}_{k}"] = int(far.sum())
        if far.any():
            print(side, k, "far verts", far.sum(), "islands", np.unique(lab[far]).tolist()[:10], "max dist cm", round(float(np.linalg.norm(g.pos[far] - ref, axis=1).max()) * 100, 1))
        if k == "hand_joint":
            for j in hc:
                n = int((far & (W[:, j] > MIN_WEIGHT)).sum())
                if n:
                    bones[g.names[j]] = n
            for r, n in regions(far).items():
                places["hand"][r] += n
    # arm bones: distance to the whole chain, so a sleeve around the arm stays out of it
    chain = [ni[PFX + side + n] for n in ("Arm", "ForeArm", "Hand") if PFX + side + n in ni]
    segs = [(g.jpos[a], g.jpos[b]) for a, b in zip(chain, chain[1:])]
    d = np.min([seg_dist(g.pos, a, b) for a, b in segs], axis=0)
    arm_j = chain[:2]
    far = (W[:, arm_j].sum(1) > MIN_WEIGHT) & (d > ARM_FAR * H)
    out[f"{side}_arm_far"] = int(far.sum())
    if far.any():
        print(side, "arm far verts", far.sum(), "islands", np.unique(lab[far]).tolist()[:10], "max dist cm", round(float(d[far].max()) * 100, 1))
    for j in arm_j:
        n = int((far & (W[:, j] > MIN_WEIGHT)).sum())
        if n:
            bones[g.names[j]] = bones.get(g.names[j], 0) + n
    for r, n in regions(far).items():
        places["arm"][r] += n
sizes = np.bincount(lab)
out["islands"] = int(len(sizes))
out["largest_island_share"] = round(float(sizes.max() / sizes.sum()), 4)
out["one_island"] = glbio.one_island(lab)
print("PROBE", json.dumps(out))
if json_out:
    json.dump(dict(out, bones=bones, places=places, height=round(H, 4), verts=len(g.pos), tris=len(g.idx), joints=len(g.names)), open(json_out, "w"), indent=1)
