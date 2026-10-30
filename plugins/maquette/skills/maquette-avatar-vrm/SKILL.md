---
name: maquette-avatar-vrm
description: Export a rigged Maquette character as a VRM 1.0 avatar into the project's exports folder. Use when asked to make a VRM, check a character's VRM readiness, map its bones when the joint names are unclear, fill its VRM meta or licence, or check how its cape, hair or sleeves hold up when the arms are lifted to the T-pose.
---

# Maquette avatar (VRM)

A character's `.vrm` is a derived export: `exports/<asset>.vrm` in the project folder, never a new
version. Tools are Maquette MCP tools (`/docs/agents/mcp-tools/`); the person's side is the app's
Avatar view (`/docs/views/avatar/`).

## Hard rules

- New weights and a rest pose change each need a person's yes in the app. `build_avatar`
  answers `waiting for approval` with `step` (`reweight`, then `rest pose`) and the before and
  after sheet paths; no tool approves either. Tell the person which step waits, then call
  `get_avatar` or `build_avatar` again later. NEVER loop on it fast.
- `denied` means the person turned the pose down. Ask what to change (for example `elbows`
  `keep` or `fingers` `keep`) before building again. A no to the weights is not a stop: the build lifts the arms
  from the model's own weights.
- Never guess bones past the readiness. When `readiness.naming.confident` is false (a
  `bone-naming` finding says "Bone names are unclear"), ask the person which joint is which, or
  set the bones you are sure of with `set_avatar` `bones` and tell them exactly what you set.
- Never change skin weights yourself. Cloth or hair weighted to the arms (the `arm-weights`
  finding) is freed only by the build's reweight step, which the person approves. Skin blockers
  (weights that do not add up to 1, vertices without weight, a joint index past the skin, a joint
  twice, a loop in the node tree) are for the person to fix in Blender; report them.
- Never capture the screen. Blender runs headless.

## Workflow

1. **Readiness.** `get_avatar` `{ project, asset }`. Read `readiness.findings` (blockers first),
   `needsReweight`, `needsNormalize`, `metaProblem`, the last `reweighted` weights, the last
   `normalized` pose and `export`. A blocker that is not
   about the rest pose (no rig, a skin blocker) needs the model fixed first; report it.
2. **Bones.** `readiness.naming` says which naming the joints matched (`convention`), how many
   required bones it found, any `ambiguous` matches and whether the map is `confident`;
   `readiness.bones` gives each bone's joint and `naming` (`descriptive` is a guess from words
   such as Left elbow; `override` is set by hand); `readiness.joints` lists the joints to pick
   from. When bones are missing or unclear, ask the person, or map them with `set_avatar`
   `{ bones: { leftUpperLeg: "Left thigh" } }` (null leaves a bone unmapped, `clearBones` maps it
   by name again) and say which you set. Read `get_avatar` again to see the readiness follow.
3. **Meta.** `set_avatar` `{ project, asset, meta?, bones?, clearBones?, material? }`. Only the
   fields you give change. A build needs `meta.name` and at least one of `meta.authors`. Ask the
   person for the licence (`avatarPermission`, `commercialUsage`, `modification` and the allow
   flags) rather than loosening the workspace defaults yourself.
4. **Plan.** `build_avatar` `{ project, asset, dryRun: true }`: the version read, whether it
   frees the arms and normalises, whether that is cached, where it writes.
5. **Free the arms.** `build_avatar` `{ project, asset, elbows?, fingers?, reweight? }` (`reweight` `auto`,
   the default, or `off`). When cloth or hair hangs on the arms, Blender first moves its weight
   to the trunk on a copy, keeping the arms, sleeves and gloves, and the call answers
   `waiting for approval` with `step: "reweight"` and `reweight.sheets` (rest, arms level, arms
   raised). Ask the person to check that a cape or hair hangs still as the arms lift and the
   sleeves follow, and approve in the Avatar view. Cloth sewn to a sleeve still stretches a little
   between arm and body; say so if the counts or sheets show it.
6. **Lift the arms.** `build_avatar` again. A model already in VRM's rest pose is written at once.
   Otherwise Blender normalises a copy (from the approved weights, when there are any) and the
   call answers `waiting for approval` with `step: "rest pose"` and `pose.sheets`. Ask the person
   to look at the sheets, especially the shoulders and the hands row at the bottom, and approve
   there. `fingers` `straighten` (the default) lays the fingers straight as far as the hand's mesh
   holds (`pose.straightened` says how far per hand, or why a hand was skipped); `keep` leaves
   them curled. A `finger-bones` finding means a hand's finger bones sit bunched at the wrist: the
   person needs to rig the hands again.
7. **Result.** Once approved, `build_avatar` again (both copies are cached, Blender does not run
   again) answers `written` with `path`, `bytes`, `unresolved`, `report` and `roundTrip`.
8. **Report.** Every build writes `exports/<asset>.vrm.report.json` beside the `.vrm`: the source
   version and its SHA-256, the material, elbows and fingers modes, what the reweight moved (`reweight`,
   when one was applied), the readiness before and after with
   every finding, `unresolved`, the bone map with each bone's `source` (`convention`, `auto` or
   `override`), the meta, the writer and `roundTrip`. Read it, then tell the person the path and
   everything in `unresolved`, blockers first, in plain words. If `roundTrip.passed` is false,
   say the file does not read back as a VRM and list `roundTrip.problems`; do not hand that file
   on. The person sees the file in the Avatar view's Preview tab.

## The scripts

`scripts/avatar_reweight.py` (frees cloth and hair from the arms) and
`scripts/avatar_normalize.py` (the rest pose) are the scripts Maquette runs, for checking a model
by hand outside the app. `scripts/vrm_addon_check.py` imports a written `.vrm` with the VRM Add-on for
Blender, headless, as a second opinion (spec, required bones, meta, left arm direction); it needs
the add-on installed and is never part of a build. `scripts/README.md` says how to run each. Work on a copy in a scratch folder, never
in the workspace's versions or a game repo.
