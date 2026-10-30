---
title: Avatar
description: Export a rigged character as a VRM 1.0 avatar, with a person's yes to its freed arms and its new rest pose.
sidebar:
  label: Avatar
---

Avatar turns a rigged character into a VRM 1.0 file for VRM apps and games. It lists the project's characters that have a model; pick one, or open it from the rig check in [Motion](/docs/views/motion/#rig-check) (**Open Avatar**). It has two tabs, Export and Preview. Springs and Expressions are planned.

## VRM readiness

The same readiness the [Inbox](/docs/views/inbox/#vrm-readiness) reads, every finding listed with the blockers first: the bones VRM requires and how clear their names are, the skin, the rest pose (facing, arms, ground, centre, symmetry, scale) and cloth or hair weighted to the arms. Nothing is hidden. A pose a build can fix says so, and so does cloth a build can free.

The skin checks cover every skinned mesh and primitive. They block the export when a vertex uses a joint the skin does not have, carries no weight, or has weights that do not add up to 1 (off by more than 0.01), when a skin lists a joint twice, when the node tree loops, or when a node is mirrored, scaled unevenly or sheared. They warn about more than 4 influences on a vertex (VRM runtimes keep 4), arm, leg or finger bones with almost no length, and left and right limbs that differ, such as a finger mapped on one side only.

## Bone map

Which joint each of the 55 VRM humanoid bones maps to, the 15 required first, then the rest of the body and the fingers. Bones map by name: VRM's and VRoid's own names, Mixamo, Unreal and Tripo's rig, then descriptive names such as `Left elbow`, `Upper arm L` or `ThumbProximal_L` (elbow is the lower arm, knee the lower leg, wrist the hand, ankle the foot). A descriptive name is a guess: it only fills a bone no exact name did, and only when one joint reads as that bone. Each row says where its joint came from: **auto** with the naming it matched by, or **set by you**.

When the names leave the map unclear (required bones missing from a rig whose joints look humanoid, or two joints that both match one bone), the readiness says **Bone names are unclear** and nothing more is guessed. Pick the joint yourself: the pencil on a row opens a search over the model's joints. **Automatic, by name** drops your choice, **Leave unmapped** leaves the bone out. A pick is saved at once with the project (`avatar.json`) and the readiness reads again; the export and the rest pose use it too.

## Meta

The VRM meta a build writes: the name, an optional version, the authors (separated by commas), copyright and contact, and the licence: who may play the avatar, commercial use, credit, changes, the yes or no uses and an other licence URL on top of VRM's own. A new avatar starts from [Settings > Avatar defaults](/docs/views/settings/#avatar-defaults) and is saved with the project (`avatar.json`) on the first change. A build saves first. It needs a name and at least one author.

## Rest pose

VRM wants a T-pose facing +Z, arms level, soles on the ground, hips on the centre. The table shows how far the model is from that. A build gets there in two steps, each headless in Blender on a copy, each waiting for your yes. The model's versions are never touched.

### Step 1: Free cloth and hair from the arms

A cape, a long coat or hair often carries weight from the arm bones, so lifting the arms drags it up like a wing. When the readiness finds cloth or hair on the arms, a build first moves that weight to the trunk. It keeps what wraps the arm: the arm itself, a sleeve, a glove, a cuff, the hand. It reads the mesh in slices along each arm, keeps the loops that hug the bone and follow on from the ones before, and moves everything else to the spine and chest, with a soft band where the two meet so nothing tears. Only the weights change: the mesh, the bones and the materials stay as they are, with at most 4 influences on a vertex.

It makes two sheets, before and after, each with the model at rest, the arms level and the arms raised, seen from the front, the side, the back and close on the shoulders. On the after sheet a cape or hair should hang still while the arms lift, and the sleeves should follow the arms without pinching at the shoulder. Cloth sewn to a sleeve (one piece of mesh from the cuff to the hem) cannot be cut there: it still stretches between the arm and the body, softer than before.

- **Approve and go on** says yes to these exact weights and lifts the arms from them.
- **Deny** keeps the model's own weights and lifts the arms as they are.

### Step 2: Lift the arms to a T-pose

When the pose is not there yet, a build normalises it: it turns the model, lifts the arms, and by default straightens the elbows (**Elbows** > **Keep** leaves them bent). It starts from the freed weights when you approved them.

The normalise makes two pose sheets, before and after, each in four poses (at rest, arms down, arms up, a step) seen from the front, the side, the back and close on the shoulders. The new pose waits for your yes:

- **Approve and write** says yes to this exact result and writes the file.
- **Deny** writes nothing. Change the elbows or fix the model, then build again.

Click a sheet to see it up close, or **Compare the shoulders** for the before and after shoulders side by side. Look at the armpits and whatever hangs by the arms.

The yes or no is only ever given here. An agent's build waits for it, and a card floats over the other views: **Open the sheets** brings you here, **Deny** turns the weights or the pose down from the card.

## Export

- **Material.** MToon soft (toon shading from the base color, no outline, alike in every VRM app) or PBR (the model's own materials). New avatars take the workspace default.
- **Output.** `exports/<asset>.vrm` in the project folder. A derived export: the asset gains no version, and each build replaces the file.
- **Last export.** Its size, when it was written, the version it came from, whether the arms were freed and the rest pose normalised, the material, and what the file still lacks, ranked. **Reveal** shows it in the file manager; **Copy path** copies its full path.
- **Round trip.** Every written file is read back at once and checked the way a VRM app would trip on it: it opens as valid glTF, it says VRM 1.0, every humanoid bone points at a node that exists and no node twice, the 15 required bones are there, each bone sits under the one VRM puts above it, the meta has a name, an author and the licence URL, `extensionsUsed` lists every extension the file uses, there are no animations, and every texture an MToon material points at is there. A pass says how many bones it read; a failure is a blocker at the top of what the file still lacks, with each problem.
- **Report.** `exports/<asset>.vrm.report.json`, written beside the file by every build: the source version and the SHA-256 of its model, when it was written, the material and elbows modes, what the reweight moved when one was applied, the readiness before (the source) and after (the written file) with every finding, what the file still lacks, the bone map with where each bone came from (`convention`, the naming most joints use; `auto`, another naming or a descriptive guess; `override`, set by you), the meta, the writer's version and the round trip. **Reveal report** shows it in the file manager.

A model already in VRM's rest pose is written straight away, without Blender.

## Preview

The written `.vrm`, loaded with [three-vrm](https://github.com/pixiv/three-vrm) the way a VRM app loads it, in the same studio light as the other previews. Drag to orbit, scroll to zoom. The chips over the stage say what three-vrm found: the spec version, the humanoid bones of 55, the required ones of 15, the spring joints and the expressions. A required bone it did not find is named under the stage.

- **Pose.** **Arms down** lowers the arms from the T-pose through the humanoid's normalised bones, which also shows the bone map works; **T-pose** shows the rest pose as written.
- **Breathe.** A slow idle in the chest and arms, so the model does not stand still. Off when your system asks for reduced motion.
- **Look at pointer.** The eyes follow the pointer when the file can move them: with eye bones, or with look expressions. A file whose look at is driven by expressions it does not have yet keeps its eyes still, and the panel says so.

Spring bones and expressions run every frame once a file has them. Nothing written yet, or the file gone, and Preview points you back to Export. A failed round trip shows over the stage too.

## Agents

[`get_avatar`](/docs/agents/mcp-tools/#get_avatar), [`set_avatar`](/docs/agents/mcp-tools/#set_avatar) and [`build_avatar`](/docs/agents/mcp-tools/#build_avatar) do the same over MCP. `build_avatar` answers `waiting for approval` with the step (`reweight` or `rest pose`) and the sheet paths while one waits; no tool approves it. `reweight: "off"` skips step 1. Once written it answers with the report's path and the round trip, and an agent tells you what the file still lacks. When the bone names are unclear, an agent asks you which joint is which, or sets the bones with `set_avatar` and tells you what it set; the Bone map shows them as set by you.
