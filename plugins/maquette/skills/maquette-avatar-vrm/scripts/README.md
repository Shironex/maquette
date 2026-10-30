# Scripts

`avatar_normalize.py` is a copy of the script Maquette embeds
(`crates/maquette-core/src/avatar/avatar_normalize.py`); a test keeps the two the same. It normalises
a rigged character's rest pose for VRM 1.0 on a copy of the model:

1. turns it so the face points to +Z and the character's left lands on +X;
2. centres the hips on the Y axis and puts the soles on Y = 0;
3. lifts each arm at the shoulder until upper arm to hand is level along X (the clavicles stay as
   they are; `"elbows": "straighten"`, the default, levels the lower arm too, and `"keep"` leaves a
   bent elbow bent);
4. with `"fingers": "straighten"`, the default, lays each finger straight along its hand in the
   palm plane, keeping half of its spread, and the thumb 45 degrees between the hand and the front;
   per hand it tries the whole turn, then 0.75, 0.5 and 0.25 of it, and keeps the first where the
   finger-weighted edges stretch at most 1.35 at the 95th percentile. A hand whose finger bones
   sit bunched at the wrist is skipped. `"keep"` leaves the fingers as they are;
5. makes that pose the rest: the armature modifier is applied, the pose becomes the rest, the
   modifier is added back; the vertex groups are kept, at most 4 influences, weights summing to 1;
6. exports a plain GLB without animations, a JSON report with the measures before and after (the
   finger curl too) and how far each hand straightened, and two pose sheets (rest, arms down, arms
   raised, a forward step; front, side, back and the shoulders; a last row with each hand from its
   thumb side and from above).

It runs headless only, never with a window, from a scratch folder:

```
blender -b --factory-startup --python-exit-code 1 --python avatar_normalize.py -- --job job.json
```

`job.json` names the model, the outputs (relative paths resolve against the job file's folder) and
the bone roles, each VRM humanoid bone name to the joint it maps to, as the VRM readiness reads
them:

```json
{
  "input": "/abs/path/model.glb",
  "output": "normalized.glb",
  "report": "report.json",
  "sheets": { "before": "sheet_before.png", "after": "sheet_after.png" },
  "bones": { "hips": "mixamorig:Hips", "leftUpperArm": "mixamorig:LeftArm" },
  "elbows": "straighten",
  "fingers": "straighten"
}
```

The bone map needs at least the hips, both upper legs, and both upper arms, lower arms and hands.

A new rest pose needs a person's yes: show the two sheets and the report before the normalised
model is used for anything. The script never changes weights. When the sheets show a cape, hair or
other cloth following the arms up, that is a weight problem of the rig itself (the same cloth moves
with the arms in the before sheet's raised row); free it with `avatar_reweight.py` first, have the
person approve, then normalise the reweighted copy.

## Freeing cloth and hair from the arms

`avatar_reweight.py` is a copy of the script Maquette embeds
(`crates/maquette-core/src/avatar/avatar_reweight.py`); a test keeps the two the same. Auto rigs
often weight a cape, a long coat or hair to the arm bones, so lifting the arms drags it up. The
script moves that weight to the trunk on a copy:

1. cuts the mesh in slices along each arm and keeps the loops that hug the bone and carry on
   from slice to slice: the arm, a sleeve, a glove; a separate glove or a cuff goes with the arm
   whole;
   the hand always stays with the arm: its own surface, reached over the mesh edges from the arm
   at the wrist through what lies past it near the bones (fingers, nails, a glove), and a small
   piece of its own lying on it or weighted mostly to it (a ring, a held prop); hair or cloth
   that only hangs beside the hand joins it through the head or the body, so it is freed even
   when the rig weighted it to the fingers;
2. moves the arm weight of every other vertex to the spine, chest and upper chest (and the
   shoulder above the armpit), with a smooth band where the two meet so cloth sewn to a sleeve
   stretches instead of tearing;
3. keeps at most 4 influences per vertex, summing to 1, and changes only `JOINTS_0` and
   `WEIGHTS_0`: the mesh, UVs, textures, bones and rest pose stay byte for byte;
4. writes the reweighted GLB, a JSON report (vertices moved, kept as sleeve, the band, from and to
   which bones, the limb radii) and two sheets, before and after, in rows rest, arms level and
   arms raised, columns front, side, back and the shoulders.

Run it headless only, never with a window, from a scratch folder:

```
blender -b --factory-startup --python-exit-code 1 --python avatar_reweight.py -- --job job.json
```

`job.json` is the same shape as the normalise's, without `elbows` and `fingers`:

```json
{
  "input": "/abs/path/model.glb",
  "output": "reweighted.glb",
  "report": "report.json",
  "sheets": { "before": "sheet_before.png", "after": "sheet_after.png" },
  "bones": { "hips": "mixamorig:Hips", "leftUpperArm": "mixamorig:LeftArm" }
}
```

New weights need a person's yes too. On the after sheet a cape or hair should hang still while the
arms lift and the sleeves should follow without pinching. A cape sewn to a sleeve (one piece of
mesh from cuff to hem) still stretches between arm and body, softer than before; only remodelling
the mesh fixes that.

## Spring chains for hair, cloth and charms

`avatar_springs.py` is a copy of the script Maquette embeds
(`crates/maquette-core/src/avatar/avatar_springs.py`); a test keeps the two the same. It runs on a
model already in VRM's rest pose (the normalised copy) in two modes.

`"mode": "find"` proposes the parts that hang off the body and changes nothing: it measures the
body as a capsule around each humanoid bone, groups the vertices well outside it into parts,
keeps the ones that hang down from where they meet the body, and writes each with an id, kind
(`hair`, `cloth` or `accessory`), root bone, centre line, length, vertex count, confidence and a
sheet with each part in its own colour:

```json
{
  "mode": "find",
  "input": "/abs/path/normalized.glb",
  "candidates": "candidates.json",
  "sheets": { "candidates": "candidates.png" },
  "bones": { "hips": "mixamorig:Hips", "head": "mixamorig:Head" }
}
```

`"mode": "chain"` adds a chain of joints along each confirmed part (its id, root bone and line
from the candidates, and `joints`, by length when left out) and weights the part to it, fading
into its old weights where it hangs so nothing tears. Only new joints, the skin's joints and
inverse bind matrices, `JOINTS_0` and `WEIGHTS_0` change, with at most 4 influences summing to 1.
The report also gives the colliders measured from the body (head, neck, chest, hips, arms, legs).
The sheets show the chains before, and a sway test after (rest, swung left, right and back):

```json
{
  "mode": "chain",
  "input": "/abs/path/normalized.glb",
  "output": "springs.glb",
  "report": "report.json",
  "sheets": { "before": "sheet_before.png", "after": "sheet_after.png" },
  "bones": { "hips": "mixamorig:Hips", "head": "mixamorig:Head" },
  "springs": [
    {
      "id": "hair-1",
      "rootBone": "head",
      "line": [
        [0, 1.5, -0.1],
        [0, 1.0, -0.2]
      ]
    }
  ]
}
```

Run either headless only, never with a window, from a scratch folder:

```
blender -b --factory-startup --python-exit-code 1 --python avatar_springs.py -- --job job.json
```

New chains need a person's yes: on the sway test each part should bend smoothly and stay on
where it hangs. It swings through the body there; the colliders only act in a VRM app. The script
writes no `VRMC_springBone`; Maquette writes it from the report when the VRM is built.

## Second opinion from the VRM Add-on

`vrm_addon_check.py` imports a written `.vrm` with the VRM Add-on for Blender (MIT, by the VRM
Consortium; not vendored here) and prints one JSON object: the spec version, how many humanoid bones
and of the 15 required ones the add-on found, the meta, and the left arm's direction from the upper
arm to the hand (in Blender's axes and in glTF's, where a T-pose reads `[1, 0, 0]`). It exits 1 when
the import fails or a required bone is missing. It is a check by hand, not part of the app or of CI.

Install the add-on once, as a Blender extension, headless:

```
blender -b --online-mode --command extension sync
blender -b --online-mode --command extension install --sync -e vrm
```

or in Blender: Edit > Preferences > Get Extensions, search VRM, Install. To keep it out of your own
Blender set up, set `BLENDER_USER_RESOURCES` to a scratch folder for both the install and the run.

Then run it headless, never with a window, on a copy of the file:

```
blender -b --factory-startup --python-exit-code 1 --python vrm_addon_check.py -- /abs/path/avatar.vrm
```

The script enables the add-on itself, so `--factory-startup` is fine.
