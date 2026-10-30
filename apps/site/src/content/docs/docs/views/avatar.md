---
title: Avatar
description: Export a rigged character as a VRM 1.0 avatar, with a person's yes to its new rest pose.
sidebar:
  label: Avatar
---

Avatar turns a rigged character into a VRM 1.0 file for VRM apps and games. It lists the project's characters that have a model; pick one, or open it from the rig check in [Motion](/docs/views/motion/#rig-check) (**Open Avatar**). It has one tab, Export. Springs, Expressions and Preview are planned.

## VRM readiness

The same readiness the [Inbox](/docs/views/inbox/#vrm-readiness) reads, every finding listed with the blockers first: the bones VRM requires, the rest pose (facing, arms, ground, centre, symmetry, scale) and cloth or hair weighted to the arms. Nothing is hidden. A pose a build can fix says so.

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

[`get_avatar`](/docs/agents/mcp-tools/#get_avatar), [`set_avatar`](/docs/agents/mcp-tools/#set_avatar) and [`build_avatar`](/docs/agents/mcp-tools/#build_avatar) do the same over MCP. `build_avatar` answers `waiting for approval` with the sheet paths while the pose waits; no tool approves it.
