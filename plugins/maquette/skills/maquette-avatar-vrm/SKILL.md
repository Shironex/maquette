---
name: maquette-avatar-vrm
description: Export a rigged Maquette character as a VRM 1.0 avatar into the project's exports folder. Use when asked to make a VRM, check a character's VRM readiness, map its bones when the joint names are unclear, fill its VRM meta or licence, check how its cape, hair or sleeves hold up when the arms are lifted to the T-pose, or make its hair, cloth or charms swing with spring bones.
---

# Maquette avatar (VRM)

A character's `.vrm` is a derived export: `exports/<asset>.vrm` in the project folder, never a new
version. Tools are Maquette MCP tools (`/docs/agents/mcp-tools/`); the person's side is the app's
Avatar view (`/docs/views/avatar/`).

## Before you start

Call `get_started` first. It says whether a workspace is open, which tools work, which keys are saved
(true or false, never a value), every project with its stage, the gates only a person can pass, and
a `next` list. No project to work in: `create_project` makes one and spends nothing. If the Maquette
tools fail with `ECONNREFUSED`, the app is closed or its server is off: ask the person to open
Maquette and turn the server on in Settings > Agents. An error names its `code`, and `fix` and
`tryTool` when they help; `personOnly` means ask the person instead of trying again.

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
- Never confirm springs on your own judgement alone. The finder only proposes parts; confirm
  the ones the person picked, or that clearly hang (a ponytail, cape panels), and say which you
  confirmed and why. A rigid part (a hood brim, a sewn on patch) also looks like a candidate. The
  chains need the person's yes in the app too (`step: "springs"`).
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
3. **Meta.** `set_avatar` `{ project, asset, meta?, bones?, clearBones?, material?, highlights? }`.
   Only the fields you give change. With `mtoon-soft`, `highlights: "soften"` (the default) softens
   strong highlights baked into dark material on a copy of the texture; `keep` leaves it. A build needs `meta.name` and at least one of `meta.authors`. Ask the
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
   between arm and body; say so if the counts or sheets show it. `reweight.bridges` names faces
   fused between an arm and the body far from the shoulder that stretch whatever the weights;
   they are taken out of the model by default (`reweight.hiddenFaces`). `set_avatar`
   `bridges: "keep"` keeps them, and the next build reweights again for a new yes.
6. **Lift the arms.** `build_avatar` again. A model already in VRM's rest pose is written at once.
   Otherwise Blender normalises a copy (from the approved weights, when there are any) and the
   call answers `waiting for approval` with `step: "rest pose"` and `pose.sheets`. Ask the person
   to look at the sheets, especially the shoulders and the hands row at the bottom, and approve
   there. `fingers` `straighten` (the default) lays the fingers straight as far as the hand's mesh
   holds (`pose.straightened` says how far per hand, or why a hand was skipped); `keep` leaves
   them curled. A hand whose finger bones sit bunched at the wrist is repaired first when its mesh
   shows five fingers (`pose.repaired` per hand); a `finger-bones` finding left after means it did
   not, and the person needs to rig the hands again.
7. **Result.** Once approved, `build_avatar` again (both copies are cached, Blender does not run
   again) answers `written` with `path`, `bytes`, `unresolved`, `report` and `roundTrip`.
8. **Springs.** Once the rest pose is approved (`springs.ready`), a build with no confirmed
   springs looks for the parts that could swing: `get_avatar` lists them in `springs.found` with
   an id, kind (`hair`, `cloth`, `accessory`), root bone, length, vertices and `confidence`, and
   `springs.found.sheet` draws each in its colour. Small pieces close to the body (a charm on a
   belt or jacket, a short chain, an earring) come after the rest as `charm-N` with `small: true`
   and a close look on the sheet; confirmed by id they keep a short chain of 2 to 4 joints and the
   `stiff` preset. Look at the sheet, ask the person which should
   swing, then call `set_avatar` with `springs`, for example
   `[{ "id": "hair-1" }, { "id": "cloth-1", "preset": "heavy", "joints": 6 }]` (`"none"`
   confirms no springs; `[]` only clears the picks, and when the last export had springs a build
   then answers `candidates waiting for a pick` and writes nothing; `stiffness`, `gravityPower`,
   `dragForce` and `hitRadius` tune one). The next `build_avatar` adds the chains on a copy and
   answers `waiting for approval` with `step: "springs"` and `chained.sheets` (the chains, and a
   sway test swung left, right and back). Ask the person to check that each part bends smoothly
   and stays on where it hangs, and approve in the Springs tab; on the sheet a part swings through
   the body, since the colliders only act in a VRM app. After the yes the file carries
   `VRMC_springBone` and `written` counts `springs`; after a no it is written without.
   `springs: "off"` on a build skips all of it. The person can wiggle the springs in the Preview
   tab. After the model changes (a new reweight or rest pose) a build carries a confirmed spring
   over to the part on its line when the finder names it differently, and lists it in `carried`;
   when nothing matches clearly it answers `springs no longer match` with `stale`: show the
   person the candidates sheet and pick again.
9. **Report.** Every build writes `exports/<asset>.vrm.report.json` beside the `.vrm`: the source
   version and its SHA-256, the material, elbows and fingers modes, what the reweight moved (`reweight`,
   when one was applied), the readiness before and after with
   every finding, `unresolved`, the bone map with each bone's `source` (`convention`, `auto` or
   `override`), the meta, the writer and `roundTrip`, plus the springs written (each with its
   joints and swing, and the collider groups), and with MToon `highlights` (how much of each
   texture was baked highlights and whether a softened copy was written). Read it, then tell the person the path and
   everything in `unresolved`, blockers first, in plain words. If `roundTrip.passed` is false,
   say the file does not read back as a VRM and list `roundTrip.problems`; do not hand that file
   on. The person sees the file in the Avatar view's Preview tab.

## The scripts

`scripts/avatar_reweight.py` (frees cloth and hair from the arms) and
`scripts/avatar_normalize.py` (the rest pose) and `scripts/avatar_springs.py` (finds the
dangling parts and adds the chains of the confirmed ones) are the scripts Maquette runs, for checking a model
by hand outside the app. `scripts/vrm_addon_check.py` imports a written `.vrm` with the VRM Add-on for
Blender, headless, as a second opinion (spec, required bones, meta, left arm direction); it needs
the add-on installed and is never part of a build. `scripts/README.md` says how to run each. Work on a copy in a scratch folder, never
in the workspace's versions or a game repo.
