"""probe.py: count vertices far from a hand that still carry that hand's (Hand + finger) weight.

Usage: blender -b --factory-startup -P probe.py -- file.glb [report.json]
For each side, vertices with hand-chain weight > 0.05 and farther than 15 cm from the Hand joint
are 'far'. If a fixweights report says a limb's chain is misplaced, the distance is also measured
from where the mesh's wrist really is (the registered Hand joint), which is the meaningful number.
"""
import sys, os, json
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np, glbio
ARGS = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:]
g = glbio.load(ARGS[0]); W = glbio.dense(g); lab, _ = glbio.components(g)
rep = json.load(open(ARGS[1])) if len(ARGS) > 1 else {"limbs": {}}
PFX = next(n for n in g.names if n.endswith("Hips"))[:-4]
out = {}
for side in ("Left", "Right"):
    hc = [i for i, n in enumerate(g.names) if n.startswith(PFX + side + "Hand")]
    m = W[:, hc].sum(1)
    hj = g.jpos[g.names.index(PFX + side + "Hand")]
    refs = {"hand_joint": hj}
    rj = rep["limbs"].get(side + "arm", {}).get("registered_joints")
    if rj:
        refs["mesh_wrist"] = np.array(rj[PFX + side + "Hand"])
    for k, ref in refs.items():
        far = (m > 0.05) & (np.linalg.norm(g.pos - ref, axis=1) > 0.15)
        out[f"{side}_{k}"] = int(far.sum())
        if far.any():
            print(side, k, "far verts", far.sum(), "islands", np.unique(lab[far]).tolist()[:10], "max dist cm", round(float(np.linalg.norm(g.pos[far] - ref, axis=1).max()) * 100, 1))
print("PROBE", json.dumps(out))
