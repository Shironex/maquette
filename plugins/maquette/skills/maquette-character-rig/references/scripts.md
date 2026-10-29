# Scripts

All scripts run headless: `blender -b --factory-startup -P <script> -- <args>`. They write files
only. Run them from a scratch folder, never from inside the workspace or a game repo. Each script
header says how it works. The humanoid scripts also run under Blender's bundled Python without
Blender (`<python> -I fixweights.py in.glb out.glb`), since they need only numpy.

## humanoid/ (Mixamo named rig, one skinned primitive)

| Script          | Arguments                                             | What it does                                                                                                                                 |
| --------------- | ----------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `glbio.py`      | (library)                                             | Tiny numpy GLB reader and patcher the others import. `glbio.save_weights` changes only the JOINTS_0 and WEIGHTS_0 bytes.                     |
| `probe.py`      | `file.glb [report.json]`                              | Counts vertices that carry hand or finger weight but sit far from the hand. With a fixweights report it also measures from the mesh's wrist. |
| `fixweights.py` | `in.glb out.glb [--debug prefix]`                     | Re-skins per connected island so cloth and hair follow the body. Writes `out.glb.report.json`.                                               |
| `rigmove.py`    | `in.glb report.json out.glb`                          | OPTIONAL. Moves joints of limbs the report calls misplaced and rewrites their bind matrices. Changes the rest pose: needs approval.          |
| `verify.py`     | `src.glb out.glb`                                     | Byte level and re-import checks: changed accessors, influences, weight sums, zero-weight vertices, image bytes, clips, bones.                |
| `rendertest.py` | `--glb a.glb [--glb b.glb] --out sheet.png [options]` | Pose test sheet. Options below.                                                                                                              |

`rendertest.py` options: `--poses rest,raise,forward,hand,twist,head,stride` (default all seven),
`--views front,side` (also `back`, `left`, `q34`), `--clips name:frac,name:frac` (frames from clips
by name and fraction of their range), `--labels before,after`, `--cell 300x400`, `--frame x,z,scale`
(camera centre and height window for a character that is not about 1 unit tall), `--xray`,
`--bones`, `--colors colors.npy` (per vertex debug colours). Poses are world-axis rotations about
each bone head, so they read the same whatever the bone roll. With several `--glb` the columns are
file by view and the rows are poses: that is the before and after sheet.

Assumptions: Mixamo bone names with the `mixamorig:` prefix, the model faces -Y with its left on
+X after import, a unit height of about 1.0 (Tripo's normalisation; use `--frame` otherwise), one
mesh primitive, no interleaved buffers. A tuning block at the top of `fixweights.py` holds the
tube radii and thresholds.

## creature/ (non-humanoid)

Everything per creature lives in `rigconfig.py`, a documented example (a made-up crystal spider).
Copy it to your scratch folder, edit it, then set `RIG_CONFIG_DIR` to that folder:

```
RIG_CONFIG_DIR=/scratch/spider blender -b --factory-startup -P weightmap.py -- in.glb out_prefix
```

| Script         | Arguments                    | What it does                                                                                                                                 |
| -------------- | ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `shlib.py`     | (library)                    | Import, bounds, orbit camera `shot()` with a caption; loads `rigconfig.py`. Sets the scene to 30 fps so clip times convert correctly.        |
| `groups.py`    | (library)                    | Colour helpers over `CHAINS`.                                                                                                                |
| `posing.py`    | (library)                    | Rig-agnostic posing by world-axis rotation about each bone head.                                                                             |
| `weightmap.py` | `in.glb out_prefix [chain]`  | Colours every vertex by its bone weights and draws each bone as a labelled stick, from front, side, back, top and perspective.               |
| `probe.py`     | `in.glb out.json`            | Dumps objects, bones (head, tail, parent), weight statistics, influence counts, zero-weight vertices and connected islands.                  |
| `pieces.py`    | `in.glb out.json`            | Real rigid pieces (islands welded across co-located vertices): size, centre, dominant bones, how mixed their weights are.                    |
| `piecemap.py`  | `in.glb out_prefix [n1,n2]`  | Colours each piece differently; optional vertex counts to highlight.                                                                         |
| `posetest.py`  | `in.glb outdir tag`          | Bends each chain in turn, renders rest and posed tiles, and measures per piece how far it deforms (a rigid piece should read near 0).        |
| `fix.py`       | `in.glb out.glb [dry]`       | Per piece weight repair: rigid pieces follow one bone, big pieces keep their owner's bones plus the torso with smoothed seams, mirror twins. |
| `verify.py`    | `a.glb b.glb`                | Compares two files: objects, counts, UVs, materials, images, bone names and rest, influences, rest positions.                                |
| `walkclip.py`  | `in.glb out.glb [frames=30]` | Procedural loopable in-place walk, keyed headless, for a visual proof of the rig.                                                            |

`fix.py` method in one paragraph: weld edge islands into pieces; find each piece's owner (torso, a
leg or a spike) from its summed weights, ignoring JUNK chains; small pieces become rigid (100
percent of one bone); big pieces keep only their owner's bones plus the torso plus any secondary
owner with a large share (a welded part), then a few Laplacian passes on the vertices that changed
so there is no hard cut; pieces with too much junk weight are rebuilt from their mirror twin; cap
4 influences and normalise.
