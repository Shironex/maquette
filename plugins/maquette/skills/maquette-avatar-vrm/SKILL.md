---
name: maquette-avatar-vrm
description: Export a rigged Maquette character as a VRM 1.0 avatar into the project's exports folder. Use when asked to make a VRM, check a character's VRM readiness, map its bones when the joint names are unclear, fill its VRM meta or licence, or check how its cape, hair or sleeves hold up when the arms are lifted to the T-pose.
---

# Maquette avatar (VRM)

A character's `.vrm` is a derived export: `exports/<asset>.vrm` in the project folder, never a new
version. Tools are Maquette MCP tools (`/docs/agents/mcp-tools/`); the person's side is the app's
Avatar view (`/docs/views/avatar/`).

## Hard rules

- A rest pose change needs a person's yes in the app. `build_avatar` answers
  `waiting for approval` with the before and after sheet paths; no tool approves it. Tell the
  person, then call `get_avatar` or `build_avatar` again later. NEVER loop on it fast.
- `denied` means the person turned the pose down. Ask what to change (for example `elbows`
  `keep`) before building again.
- Never guess bones past the readiness. When `readiness.naming.confident` is false (a
  `bone-naming` finding says "Bone names are unclear"), ask the person which joint is which, or
  set the bones you are sure of with `set_avatar` `bones` and tell them exactly what you set.
- Never change skin weights. Cloth or hair weighted to the arms, and skin blockers (weights that
  do not add up to 1, vertices without weight, a joint index past the skin, a joint twice, a loop
  in the node tree), are for the person to fix in Blender; report them.
- Never capture the screen. Blender runs headless.

## Workflow

1. **Readiness.** `get_avatar` `{ project, asset }`. Read `readiness.findings` (blockers first),
   `needsNormalize`, `metaProblem`, the last `normalized` pose and `export`. A blocker that is not
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
   normalises, whether that is cached, where it writes.
5. **Build.** `build_avatar` `{ project, asset, elbows? }` (`straighten`, the default, or `keep`).
   A model already in VRM's rest pose is written at once. Otherwise Blender normalises a copy and
   the call answers `waiting for approval` with `pose.sheets`. Ask the person to look at the
   sheets in the Avatar view, especially the shoulders, and approve there.
6. **Result.** Once approved, `build_avatar` again (the normalised copy is cached, Blender does not
   run again) answers `written` with `path`, `bytes`, `unresolved`, `report` and `roundTrip`.
7. **Report.** Every build writes `exports/<asset>.vrm.report.json` beside the `.vrm`: the source
   version and its SHA-256, the material and elbows modes, the readiness before and after with
   every finding, `unresolved`, the bone map with each bone's `source` (`convention`, `auto` or
   `override`), the meta, the writer and `roundTrip`. Read it, then tell the person the path and
   everything in `unresolved`, blockers first, in plain words. If `roundTrip.passed` is false,
   say the file does not read back as a VRM and list `roundTrip.problems`; do not hand that file
   on. The person sees the file in the Avatar view's Preview tab.

## The scripts

`scripts/avatar_normalize.py` is the rest pose script Maquette runs, for checking a model by hand
outside the app. `scripts/vrm_addon_check.py` imports a written `.vrm` with the VRM Add-on for
Blender, headless, as a second opinion (spec, required bones, meta, left arm direction); it needs
the add-on installed and is never part of a build. `scripts/README.md` says how to run both. Work on a copy in a scratch folder, never
in the workspace's versions or a game repo.
