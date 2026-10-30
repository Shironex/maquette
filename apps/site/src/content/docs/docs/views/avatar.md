---
title: Avatar
description: Export a rigged character as a VRM 1.0 avatar, with a person's yes to its freed arms, its new rest pose and its spring chains.
sidebar:
  label: Avatar
---

Avatar turns a rigged character into a VRM 1.0 file for VRM apps and games. It lists the project's characters that have a model; pick one, or open it from the rig check in [Motion](/docs/views/motion/#rig-check) (**Open Avatar**). It has three tabs, Export, Springs and Preview. Expressions are planned.

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

Some generated meshes fuse a few faces between a sleeve and the hip, where the sleeve rested on it (a cuff joined to a strap on the shorts). No weights keep those faces from stretching into a dark strand when the arm lifts, so the summary names them as faces joining the arm to the body. **Faces fused to the body** decides what happens to them:

- **Hide** (the default) takes only those faces out of the model, so they are not drawn. Every vertex, UV, texture and weight stays where it was, and the after sheet shows the result.
- **Keep** leaves them, stretching as the arm lifts.

Changing it reweights again, and the new weights wait for your yes.

- **Approve and go on** says yes to these exact weights and lifts the arms from them.
- **Deny** keeps the model's own weights and lifts the arms as they are.

### Step 2: Lift the arms to a T-pose

When the pose is not there yet, a build normalises it: it turns the model, lifts the arms, and by default straightens the elbows (**Elbows** > **Keep** leaves them bent) and the fingers (**Fingers** > **Keep** leaves them curled). It starts from the freed weights when you approved them.

VRM's rest pose lays the four fingers straight along the hand, palm down, and the thumb 45 degrees between the hand and the front. The table's **Most finger curl** row shows how far the most curled finger bends. Straightening keeps a little of the fingers' natural spread, and stops short when the hand's mesh would tear: it tries the whole turn, then three quarters, a half and a quarter, and keeps the first one under which no finger stretches much. Fingers modelled fused together keep some curl. A hand whose finger bones all sit bunched at the wrist, as a broken auto rig leaves them, is left as it is, with a warning to rig the hands again.

The normalise makes two pose sheets, before and after, each in four poses (at rest, arms down, arms up, a step) seen from the front, the side, the back and close on the shoulders, and a last row with each hand from its thumb side and from above. The new pose waits for your yes:

- **Approve and write** says yes to this exact result and writes the file.
- **Deny** writes nothing. Change the elbows or the fingers, or fix the model, then build again.

Click a sheet to see it up close, **Compare the shoulders** for the before and after shoulders side by side, or **Compare the hands** for both hands. Look at the armpits and whatever hangs by the arms, and at the fingertips and the webbing between the fingers.

When the arms were freed first, cloth sewn to the sleeves still follows them: what the file still lacks then says how many vertices are left on the arms after the reweight, as a note, not as something to fix.

The yes or no is only ever given here. An agent's build waits for it, and a card floats over the other views: **Open the sheets** brings you here, **Deny** turns the weights or the pose down from the card.

## Export

- **Material.** MToon soft (toon shading from the base color, no outline, alike in every VRM app) or PBR (the model's own materials). New avatars take the workspace default.
- **Baked highlights.** With MToon only. A generated model has its lighting painted into its color texture: on black leather the light's shine is a bright, nearly white patch, which toon shading shows as a white blotch. **Soften** (the default) has Blender write a copy of the texture with only strong highlights on dark material toned down toward the color around them; skin, light cloth, colored prints and large light areas stay as they are, and small silver details stay visible, only dimmer. The VRM shades from that copy; the model's own texture never changes and stays in the file, unused. A texture with almost no highlights is written as it is. **Keep** writes the texture as it is.
- **Output.** `exports/<asset>.vrm` in the project folder. A derived export: the asset gains no version, and each build replaces the file.
- **Last export.** Its size, when it was written, the version it came from, whether the arms were freed and the rest pose normalised, the material, how much of the texture was softened as baked highlights, and what the file still lacks, ranked. **Reveal** shows it in the file manager; **Copy path** copies its full path.
- **Round trip.** Every written file is read back at once and checked the way a VRM app would trip on it: it opens as valid glTF, it says VRM 1.0, every humanoid bone points at a node that exists and no node twice, the 15 required bones are there, each bone sits under the one VRM puts above it, the meta has a name, an author and the licence URL, `extensionsUsed` lists every extension the file uses, there are no animations, and every texture an MToon material points at is there. A pass says how many bones it read; a failure is a blocker at the top of what the file still lacks, with each problem.
- **Report.** `exports/<asset>.vrm.report.json`, written beside the file by every build: the source version and the SHA-256 of its model, when it was written, the material, elbows and fingers modes, what the reweight moved when one was applied (with the faces fused to the body it found and how many it hid), the readiness before (the source) and after (the written file) with every finding, what the file still lacks, the bone map with where each bone came from (`convention`, the naming most joints use; `auto`, another naming or a descriptive guess; `override`, set by you), the meta, the writer's version, the round trip and, with MToon, `highlights`: kept or softened, and per base color texture its size, the share of its pixels that are baked highlights (`coverage`, and `darkCoverage` within its dark areas) and whether a softened copy was written or why not. **Reveal report** shows it in the file manager.

A model already in VRM's rest pose is written straight away, without Blender.

## Springs

Spring bones let a ponytail, a cape's panels or a charm swing in a VRM app. AI made characters come with a humanoid rig only, so these parts move rigidly with the head or the chest. The Springs tab finds them and gives each one a chain of joints of its own. It reads the rest pose the VRM is written from, so it opens once that pose is ready: the model's own when it is already VRM's, else the normalised one after your yes.

### Find the parts

**Find dangling parts** reads the model headless in Blender and proposes the parts that hang off the body. It changes nothing and needs no yes. It measures the body around each humanoid bone, takes what lies well off it (and away from the head itself), joins it into parts, finds where each one hangs from, and keeps the ones that hang down. Each part shows its colour on the sheet beside the list, its kind (hair from the head or neck, cloth when long, an accessory otherwise), the bone it hangs from, its length, its vertex count and how sure the finder is. Below 60% it is marked: a rigid hood brim or a part close to the body can look like hair. A build looks once by itself when nothing is confirmed, and the result is kept for that model.

Nothing swings until you confirm it: switch on the parts that should swing, and leave a stiff brim or a sewn on patch off. Each confirmed part can take a joint count (by its length unless you set one) and a swing:

- **Soft** for hair: light drag, little gravity.
- **Heavy** for cloth: more drag and gravity.
- **Stiff** for charms and small parts: springs back quickly.

**Small pieces** lists what sits too close to the body for that: a charm on a belt or a jacket, a short chain, an earring. The finder reads where the mesh stands out of the surface around it and keeps the narrow bits that hang down, away from the forearms, hands, shins and feet. Each is a `charm` accessory from the bone it hangs from, with how sure the finder is, and the sheet adds a close look at the first few. A small piece gets a short chain of 2 to 4 joints and the stiff swing, and once its chain is made the sway test adds a near look at it. A charm drawn flat onto the cloth stays rigid, since nothing of it stands out.

**No springs** confirms that the VRM carries none, whatever is found. Switching every part off is not the same: it only means nothing is picked yet. When the last VRM had springs and nothing is picked, a build stops with "Pick the springs first" and writes nothing, so a cleared list never quietly drops a ponytail from the file. Pick the parts to keep or switch on **No springs**, then build again.

### When the model changes

A new reweight or rest pose can change how the finder reads a part: the same ponytail may come back with another id, kind or bone. A build holds each confirmed spring to the parts found now. One that still matches stays as it is. One whose part is still on its line, hanging from about the same place, is carried over to that part (its new id, bone and line) and saved, keeping your kind, joints and swing; the notice says which. When no part matches clearly, the build stops with "The confirmed springs no longer match; pick again" and writes nothing. The Springs tab lists confirmed springs no part goes by now, each with a **Drop** button.

### Make the chains

**Make chains** saves and, headless in Blender on a copy, adds a chain of joints along each confirmed part, hung from its bone, and weights the part to it: smoothly from joint to joint, blending into the bone it hangs from at the top so nothing tears where it attaches. Only new joints and the skin weights change: the humanoid bones, the rest pose, the mesh, the UVs and the textures stay as they are, with at most 4 influences on a vertex. It also measures colliders from the body (a sphere for the head, capsules for the neck, chest, hips, upper arms and upper legs) and groups them, so hair stays out of the head, chest, back and arms in a VRM app.

It makes two sheets: the parts with their joints marked, and a sway test with each chain swung left, right and back, seen from the front, the side, the back and close to where it hangs. A part should bend smoothly, stay on at the top and keep its shape. In the sway test it can pass through the body: the colliders only act in a VRM app. The chains wait for your yes:

- **Approve and write** says yes to these exact chains and writes the VRM with `VRMC_springBone`: a spring per part with its swing, the colliders and their groups.
- **Deny** writes the VRM without springs. Change the parts or their joints and make the chains again.

Changing a swing later needs no new chains: a build writes the new values. Changing the parts or their joints makes new chains, which wait for a yes again. The report records the springs, and the round trip reads them back.

## Preview

The written `.vrm`, loaded with [three-vrm](https://github.com/pixiv/three-vrm) the way a VRM app loads it, in the same studio light as the other previews. Drag to orbit, scroll to zoom. The chips over the stage say what three-vrm found: the spec version, the humanoid bones of 55, the required ones of 15, the spring joints and the expressions. A required bone it did not find is named under the stage.

- **Pose.** **Arms down** lowers the arms from the T-pose through the humanoid's normalised bones, which also shows the bone map works; **T-pose** shows the rest pose as written.
- **Breathe.** A slow idle in the chest and arms, so the model does not stand still. Off when your system asks for reduced motion.
- **Look at pointer.** The eyes follow the pointer when the file can move them: with eye bones, or with look expressions. A file whose look at is driven by expressions it does not have yet keeps its eyes still, and the panel says so.

- **Wiggle.** Shakes the hips and the head once, so the spring bones swing. Off for a file without springs.

Spring bones and expressions run every frame once a file has them. Nothing written yet, or the file gone, and Preview points you back to Export. A failed round trip shows over the stage too.

## Agents

[`get_avatar`](/docs/agents/mcp-tools/#get_avatar), [`set_avatar`](/docs/agents/mcp-tools/#set_avatar) and [`build_avatar`](/docs/agents/mcp-tools/#build_avatar) do the same over MCP. `build_avatar` answers `waiting for approval` with the step (`reweight`, `rest pose` or `springs`) and the sheet paths while one waits; no tool approves it. `reweight: "off"` skips step 1, and `springs: "off"` writes no springs. `get_avatar` lists the parts the finder found, and `set_avatar` confirms them by id; an agent confirms only the parts you or it judged from the sheet, and tells you which. Once written it answers with the report's path and the round trip, and an agent tells you what the file still lacks. When the bone names are unclear, an agent asks you which joint is which, or sets the bones with `set_avatar` and tells you what it set; the Bone map shows them as set by you.
