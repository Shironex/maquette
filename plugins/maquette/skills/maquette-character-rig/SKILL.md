---
name: maquette-character-rig
description: Take a rigged character or creature from the Maquette inbox through a rig check to a clean v2 with correct skin weights. Use when get_inbox_checks flags the rig (not Mixamo, hair or cloth weighted to finger bones, no clips), when a rigged model tears, stretches or drags cloth when posed, when a non-humanoid auto-rig has swapped Left/Right limbs or junk duplicate chains, or when clips need baking onto a rigged model.
---

# Maquette character rig

Fixes SKIN WEIGHTS (and only if approved, a misplaced joint chain) of an auto-rigged GLB with
headless Blender, proves the fix with before and after pose sheets that you LOOK at, and lands the
result as the asset's next version. The original stays as v1. Tools are Maquette MCP tools
(`/docs/agents/mcp-tools/`); scripts are in `scripts/` next to this file, usage in
`references/scripts.md`. The app's Rig doctor runs these same scripts: `start_rig_doctor`,
`run_rig_doctor` and `get_rig_doctor` give you its sheets and defects, and a person adds the result
there. Use this skill by hand when the doctor refuses a model or stops.

## Before you start

Call `get_started` first. It says whether a workspace is open, which tools work, which keys are saved
(true or false, never a value), every project with its stage, the gates only a person can pass, and
a `next` list. No project to work in: `create_project` makes one and spends nothing. If the Maquette
tools fail with `ECONNREFUSED`, the app is closed or its server is off: ask the person to open
Maquette and turn the server on in Settings > Agents. An error names its `code`, and `fix` and
`tryTool` when they help; `personOnly` means ask the person instead of trying again.

## Start with the rig doctor

`start_rig_doctor` `{ inbox | project + asset, version? }` opens a case and reads the file with no
Blender: the route (humanoid for Mixamo names, creature otherwise), what it refuses and why. If it
does not refuse, `run_rig_doctor` `{ id, step }` runs `pose-test`, then `fix`, then (once, if
needed) `round-2` on a copy, each answering at once; follow with `get_rig_doctor` `{ id }` and LOOK
at its sheets. Its numbers: stretch past the body (a row is flagged from about 80 mm) and seam gap
(from 5 mm), in mm. An inbox model must be accepted first (step 2). The result lands only when the
person adds it as the next version on the approval card in the app: no tool does that or approves
a joint move. Come back to the manual steps below when the doctor refuses a model or stops.

## Hard rules

- Blender runs headless (`-b`). Never open its window. Never capture the screen. Pictures are PNG
  files from the scripts; read them from disk.
- Never edit the inbox file or an accepted version in place. Work on a copy in a scratch folder
  outside the workspace and the repo; the result goes in through `add_model_file`.
- Keep mesh, UVs, texture, bone names, bone hierarchy and rest pose untouched. Only weights
  change. Moving a joint changes the rest pose: do it only when the person approved after seeing
  the evidence (step 5).
- At most 4 influences per vertex, normalised, no vertex without weight.
- Fix per connected island with smooth seams. Never cut weights by a hard distance threshold
  inside one piece: it tears cloth.
- Mixamo clips can be used in the person's projects but are never redistributed: no Mixamo FBX or
  clip files in any repo, issue or docs. Keep them in the workspace animation library.
- Never start a Tripo or Meshy API send. A redo that costs credits is the person's call
  (`maquette-by-hand-round`).

## 1. Read the model

1. `list_inbox`: find the model; the answer's `inbox` folder plus the item `id` is its file. Note
   `sentFor` or `aimedAt` for its asset.
2. `get_inbox_checks` `{ id }`. Read: the rig check (Mixamo skeleton or another), the hair or tail
   vertices weighted to finger bones warning (a heuristic: it is skipped for Draco or meshopt
   files), clips and their names, triangles, textures, scale. Write down what the model has and
   what the person expects (humanoid with Mixamo names, or a creature with its own chains).
3. If the rig is fine and nothing tears, stop: accept it with `maquette-by-hand-round` and say why
   nothing was fixed.

## 2. Accept the original as v1

`accept_inbox_model` `{ id, project, asset? }`. The answer's `file` is v1 in the workspace. v1 is
the safe base to compare against and to go back to. Then `list_assets` shows the asset and its
versions.

## 3. Set up a scratch folder and Blender

1. Make a scratch folder outside the workspace and the repo. Copy v1 into it as `in.glb`.
2. Find Blender: `blender` on the path, `/Applications/Blender.app/Contents/MacOS/Blender` on a
   Mac, `C:\Program Files\Blender Foundation\Blender <version>\blender.exe` on Windows. The
   scripts use numpy, which Blender's Python ships with. Run script files as
   `blender -b --factory-startup -P script.py -- args`.
3. Decide the route:
   - Humanoid, Mixamo names (`mixamorig:Hips` ...), one skinned primitive: `scripts/humanoid/`.
   - Non-humanoid (spider, crab, four legged beast), Tripo `tripo::...` or `bone_N` names, or a
     humanoid file the humanoid scripts refuse: `scripts/creature/`.

## 4. Pose test BEFORE and probe

Always test with poses, not only still renders: a fault shows when a joint moves.

Humanoid: `scripts/humanoid/rendertest.py` with `--glb in.glb --out before.png --labels before`,
default poses `rest,raise,forward,hand,twist,head,stride`, views front and side, plus frames from
every clip: `--clips <clipname>:0,<clipname>:0.5` (clip names come from `get_inbox_checks`).
`probe.py in.glb` counts vertices that still carry hand or finger weight far from the hand. Judge
the count against the hand's real size: on oversized fists (a golem, a brute) hundreds of
vertices 25 cm out are the hand itself. The pose sheet decides, not the count.

Creature: copy `scripts/creature/rigconfig.py` to your scratch folder and set
`RIG_CONFIG_DIR`. The renders need real bone names in `CHAINS`: run `autoconfig.py in.glb <scratch>`
for a first pass, or fill them from `probe.py`'s bone heads and parents and set `FRONT` from where
the head is (a Tripo quadruped can face -X). Then run `weightmap.py` (rig map renders: every bone group in its own colour from five
views), `probe.py` (bones, weights, islands), `pieces.py` and `piecemap.py` (rigid pieces and
their weight mix). READ the renders and fill the `rigconfig.py` tables: what the groups really
are, which chains are junk, which pieces are rigid. If `pieces.py` reports `WELDED_PIECES 1`, the model
is one continuous shell (fur or shards sculpted into one surface): there are no rigid pieces, so set
`WELD_PIECES = False` and `PIECE_RULES = False`. Then `posetest.py in.glb outdir before` bends every
chain into `outdir/before_sheet.png` and measures piece deformation, seam gaps and edge stretch (the
log prints where the worst edge is: that place is usually the defect).

LOOK at every sheet. Name the defects you see, by place, before changing anything. Common ones
(see `references/rig-defects.md` for how each is recognised and fixed):

- Cloth, capes, robes, hair or tails weighted to arm, hand or finger bones: they swing with the
  hands.
- A limb chain placed off its mesh (a whole arm chain inside the cape).
- Non-humanoid: Left and Right limb names that do not match the real sides; junk duplicate
  chains; rigid pieces (crystals, plates) split between bones so they bend.

## 5. Fix

One island first: if `probe.py` reports `one_island`, skip `fixweights.py` (it refuses, since a per
island fix tears sleeves and legs on one surface). Run `narrow.py in.glb out.glb` instead, and say
in your report that only a narrow tail-off-arm reweight ran.

Humanoid: `fixweights.py in.glb out.glb` re-skins per connected island (limb tubes, fused pieces,
trunk weights with a smooth geodesic seam). Only the JOINTS_0 and WEIGHTS_0 bytes change, so mesh,
UVs, texture and rest pose stay byte identical. It writes `out.glb.report.json` that lists limbs it
judged `misplaced` and where the mesh's real joints are.

If a limb is misplaced, the weights alone are fixed by mirroring the good side. Moving the joints
(`rigmove.py in.glb report.json out.glb`) changes the rest pose and clips with translation tracks
would pull them back: show the person the before sheet and the report and ask. Without a yes, keep
the weight-only result.

Creature: `fix.py in.glb out.glb dry` prints what it would do (per piece, or per `REMAP` and
`STRAY_DIST` step with where the stray weight sits) (rigid, smoothed, mirrored)
and how many vertices change; check it against what you saw, adjust `rigconfig.py`, then run it
without `dry`. It rewrites only the JOINTS_0 and WEIGHTS_0 bytes when it can (it says so), else it
exports through Blender (images re-encoded, unskinned helper meshes dropped).

Iterate until the sheets are clean. Keep the changes the sheets justify and no more.

## 6. Verify

1. `verify.py` (humanoid: `-- src.glb out.glb`; creature: same shape) reports what differs:
   changed accessors must be only JOINTS_0 and WEIGHTS_0 (humanoid), bone names, parents and rest
   heads unchanged, max influences 4, weight sums 1, zero-weight vertices 0, image bytes
   identical, animations and bones after re-import.
2. Pose test AFTER with the same poses and labels, into `after.png` (or `posetest.py ... after`).
   Put before and after side by side (humanoid: `rendertest.py` takes several `--glb`, one sheet;
   creature: `sheet.py -- both.png before_sheet.png after_sheet.png`) and LOOK:
   the hands carry hands, the cloth follows the body, nothing tears at the seams, rigid pieces
   stay rigid. Say what changed and what did not. If a defect remains, return to step 5.
3. Optional clip proof: `walkclip.py` (creatures) writes a loopable in-place walk at 30 fps onto
   a copy so the person sees the rig move; `clipsheet.py` renders its frames to one PNG to look at.
   Do not ship it unless asked.

## 7. Land v2

1. `add_model_file` `{ project, asset, path: <out.glb absolute path> }` with `accept` left false.
   The file goes into the inbox aimed at the asset (or its newest waiting By hand send).
2. `get_inbox_checks` on the returned `inboxFile`: the finger-weight warning should be gone,
   triangles and textures equal v1's. If a check got worse, stop and report.
3. `accept_inbox_model` `{ id, project, asset }` makes v2. v1 stays. Report what changed: the
   defects named, the sheet paths, and the check before and after.

## 8. Clips

- Clips the model came with: `get_inbox_checks` lists them. Keep them; if the person wants them
  reusable, `save_model_clips` `{ project, asset, pack?, dryRun? }` saves skeleton and keys to the
  animation library (use `dryRun: true` first to see the rig and clips).
- Library clips onto the model: `list_animations` `{ query?, pack?, tag?, rig? }`, then
  `use_animations` `{ project, asset, clips: [{ clip, name?, start?, end?, loops?, rootMotion?,
speed?, pinFeet? }], map?, dryRun? }`. Run with `dryRun: true` first and read the review: each
  clip is `good`, `look` (a planted foot slides more than 1.5 cm) or `failed` (wrong units,
  nothing mapped). Failed clips are left out. For `look`, set `pinFeet` on that clip and bake
  again; the review gives the slide before (`unpinnedCm`) and after (`slideCm`). When the pin slid
  more than the clip did, it is left off and the review says `pinWorseCm`: keep the unpinned clip. The bake becomes the next version; `get_retarget_review` shows the last review.
- Importing Mixamo FBX or a pack zip: `import_animations` `{ paths, singles?, keepDuplicates?,
dryRun? }`, paths inside the workspace; use `dryRun` first. Clips keep their own frame rate (30
  fps stays 30 fps) and the model exports at the highest rate among its clips.
- A non-humanoid rig has no Mixamo map: expect to give `map` by hand to `use_animations`.

## Stop conditions

- Stop and ask when: a joint move is needed, a check got worse, the fix would need a new
  texture, UV or mesh edit, or the sheets still show a defect after two rounds.
- A model the scripts refuse (several skinned meshes, interleaved buffers, no skin): say so and
  suggest redoing the generation as rigged, or fixing it by hand in Blender.
- Finish with: the asset and its versions, the defect list, before and after sheet paths, the
  checks, and anything left open.
