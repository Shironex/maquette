---
title: Avatar
description: Export a rigged character as a VRM 1.0 avatar, with a person's yes to its new rest pose.
sidebar:
  label: Avatar
---

Avatar turns a rigged character into a VRM 1.0 file for VRM apps and games. It lists the project's characters that have a model; pick one, or open it from the rig check in [Motion](/docs/views/motion/#rig-check) (**Open Avatar**). It has one tab, Export. Springs, Expressions and Preview are planned.

## VRM readiness

The same readiness the [Inbox](/docs/views/inbox/#vrm-readiness) reads, every finding listed with the blockers first: the bones VRM requires and how clear their names are, the skin, the rest pose (facing, arms, ground, centre, symmetry, scale) and cloth or hair weighted to the arms. Nothing is hidden. A pose a build can fix says so.

The skin checks cover every skinned mesh and primitive. They block the export when a vertex uses a joint the skin does not have, carries no weight, or has weights that do not add up to 1 (off by more than 0.01), when a skin lists a joint twice, when the node tree loops, or when a node is mirrored, scaled unevenly or sheared. They warn about more than 4 influences on a vertex (VRM runtimes keep 4), arm, leg or finger bones with almost no length, and left and right limbs that differ, such as a finger mapped on one side only.

## Bone map

Which joint each of the 55 VRM humanoid bones maps to, the 15 required first, then the rest of the body and the fingers. Bones map by name: VRM's and VRoid's own names, Mixamo, Unreal and Tripo's rig, then descriptive names such as `Left elbow`, `Upper arm L` or `ThumbProximal_L` (elbow is the lower arm, knee the lower leg, wrist the hand, ankle the foot). A descriptive name is a guess: it only fills a bone no exact name did, and only when one joint reads as that bone. Each row says where its joint came from: **auto** with the naming it matched by, or **set by you**.

When the names leave the map unclear (required bones missing from a rig whose joints look humanoid, or two joints that both match one bone), the readiness says **Bone names are unclear** and nothing more is guessed. Pick the joint yourself: the pencil on a row opens a search over the model's joints. **Automatic, by name** drops your choice, **Leave unmapped** leaves the bone out. A pick is saved at once with the project (`avatar.json`) and the readiness reads again; the export and the rest pose use it too.

## Meta

The VRM meta a build writes: the name, an optional version, the authors (separated by commas), copyright and contact, and the licence: who may play the avatar, commercial use, credit, changes, the yes or no uses and an other licence URL on top of VRM's own. A new avatar starts from [Settings > Avatar defaults](/docs/views/settings/#avatar-defaults) and is saved with the project (`avatar.json`) on the first change. A build saves first. It needs a name and at least one author.

## Rest pose

VRM wants a T-pose facing +Z, arms level, soles on the ground, hips on the centre. The table shows how far the model is from that. When it is not there yet, a build normalises a copy headless in Blender: it turns the model, lifts the arms, and by default straightens the elbows (**Elbows** > **Keep** leaves them bent). The model's versions are never touched.

The normalise makes two pose sheets, before and after, each in four poses (at rest, arms down, arms up, a step) seen from the front, the side, the back and close on the shoulders. The new pose waits for your yes:

- **Approve and write** says yes to this exact result and writes the file.
- **Deny** writes nothing. Change the elbows or fix the model, then build again.

Click a sheet to see it up close, or **Compare the shoulders** for the before and after shoulders side by side. Look at the armpits and whatever hangs by the arms, like a cape or hair: cloth weighted to the arms rises with them. A reweight that frees it is planned.

The yes or no is only ever given here. An agent's build waits for it, and a card floats over the other views: **Open the sheets** brings you here, **Deny** turns the pose down from the card.

## Export

- **Material.** MToon soft (toon shading from the base color, no outline, alike in every VRM app) or PBR (the model's own materials). New avatars take the workspace default.
- **Output.** `exports/<asset>.vrm` in the project folder. A derived export: the asset gains no version, and each build replaces the file.
- **Last export.** Its size, when it was written, the version it came from, whether the rest pose was normalised, the material, and what the file still lacks, ranked. **Reveal** shows it in the file manager; **Copy path** copies its full path.

A model already in VRM's rest pose is written straight away, without Blender.

## Agents

[`get_avatar`](/docs/agents/mcp-tools/#get_avatar), [`set_avatar`](/docs/agents/mcp-tools/#set_avatar) and [`build_avatar`](/docs/agents/mcp-tools/#build_avatar) do the same over MCP. `build_avatar` answers `waiting for approval` with the sheet paths while the pose waits; no tool approves it. When the bone names are unclear, an agent asks you which joint is which, or sets the bones with `set_avatar` and tells you what it set; the Bone map shows them as set by you.
