# Scripts

`avatar_normalize.py` is a copy of the script Maquette embeds
(`crates/maquette-core/src/avatar/avatar_normalize.py`); a test keeps the two the same. It normalises
a rigged character's rest pose for VRM 1.0 on a copy of the model:

1. turns it so the face points to +Z and the character's left lands on +X;
2. centres the hips on the Y axis and puts the soles on Y = 0;
3. lifts each arm at the shoulder until upper arm to hand is level along X (the clavicles and the
   fingers stay as they are; `"elbows": "straighten"` levels the lower arm too);
4. makes that pose the rest: the armature modifier is applied, the pose becomes the rest, the
   modifier is added back; the vertex groups are kept, at most 4 influences, weights summing to 1;
5. exports a plain GLB without animations, a JSON report with the measures before and after, and
   two pose sheets (rest, arms down, arms raised, a forward step; front, side, back and the
   shoulders).

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
  "elbows": "keep"
}
```

The bone map needs at least the hips, both upper legs, and both upper arms, lower arms and hands.

A new rest pose needs a person's yes: show the two sheets and the report before the normalised
model is used for anything. The script never changes weights. When the sheets show a cape, hair or
other cloth following the arms up, that is a weight problem of the rig itself (the same cloth moves
with the arms in the before sheet's raised row); fix it with a reweight the person agrees to, then
normalise again.
