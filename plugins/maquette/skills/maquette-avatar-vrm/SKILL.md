---
name: maquette-avatar-vrm
description: Export a rigged Maquette character as a VRM 1.0 avatar into the project's exports folder. Use when asked to make a VRM, check a character's VRM readiness, fill its VRM meta or licence, or check how its cape, hair or sleeves hold up when the arms are lifted to the T-pose.
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
- Never change skin weights. Cloth or hair weighted to the arms is a warning to show the person,
  not to fix.
- Never capture the screen. Blender runs headless.

## Workflow

1. **Readiness.** `get_avatar` `{ project, asset }`. Read `readiness.findings` (blockers first),
   `needsNormalize`, `metaProblem`, the last `normalized` pose and `export`. A blocker that is not
   about the rest pose (missing required bones, no rig) needs the model fixed first; report it.
2. **Meta.** `set_avatar` `{ project, asset, meta?, bones?, clearBones?, material? }`. Only the
   fields you give change. A build needs `meta.name` and at least one of `meta.authors`. Ask the
   person for the licence (`avatarPermission`, `commercialUsage`, `modification` and the allow
   flags) rather than loosening the workspace defaults yourself.
3. **Plan.** `build_avatar` `{ project, asset, dryRun: true }`: the version read, whether it
   normalises, whether that is cached, where it writes.
4. **Build.** `build_avatar` `{ project, asset, elbows? }` (`straighten`, the default, or `keep`).
   A model already in VRM's rest pose is written at once. Otherwise Blender normalises a copy and
   the call answers `waiting for approval` with `pose.sheets`. Ask the person to look at the
   sheets in the Avatar view, especially the shoulders, and approve there.
5. **Result.** Once approved, `build_avatar` again (the normalised copy is cached, Blender does not
   run again) answers `written` with `path`, `bytes` and `unresolved`. Report the path and every
   unresolved warning.

## The script

`scripts/avatar_normalize.py` is the rest pose script Maquette runs, for checking a model by hand
outside the app. `scripts/README.md` says how to run it. Work on a copy in a scratch folder, never
in the workspace's versions or a game repo.
