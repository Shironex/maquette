---
title: Animations
description: Keep animation clips once for every project, skeleton only, imported from Mixamo FBX files and pack zips or saved from a rigged model, and bake them onto rigged models.
sidebar:
  label: Animations
---

The Animations library keeps clips once, outside every project, for every project in the workspace. It sits under Libraries in the rail, next to [Models](/docs/views/library/), with the number of clips after its name. A clip is skeleton only: the bones, their rest pose and the keys, never a mesh. A rigged model takes a clip by baking it; the library itself never changes a project.

## Import clips

**Import clips** in the top bar opens a sheet. **Add files** picks Mixamo FBX files or a pack zip of them. Maquette reads them headless in Blender and shows each file before anything is added:

- **A pack zip** becomes a pack named after the zip. Each FBX inside is one clip. The sheet says how many clips it found, their rate, and the rig.
- **A single FBX** goes to Singles, or the pack you type under Single files go to.
- **The rig** is read from the bone names: Mixamo (`mixamorig:` names), Tripo, or Custom. A custom rig goes in as it is; the first model it is used on asks for a bone map.
- **The skin** is dropped. Only the skeleton and the keys are kept, so a clip is a few dozen KB.
- **A duplicate** (a clip whose keys the library has already) is skipped, or added as a clip of its own with **Keep both**.
- **A new take** of a clip already in the pack, with other keys, replaces it as its next revision. Models that baked the older take are offered an update.
- **A file with no keys** (one pose) is left out, with the reason. So is anything that is not an FBX file or a zip of them.

The sheet suggests tags from the clip names (combat, magic, locomotion, dodge, idle, hit react) and shows what was read and what the library stores. **Keep the FBX files** keeps each FBX next to its clip; it is off by default. **Import N clips** adds them.

## Save clips to library

A rigged model can give its clips to the library:

- In [Motion](/docs/views/motion/), **Add clip** > **Save clips to library** on the selected model.
- In the [Inbox](/docs/views/inbox/), **Save clips to library** on a model that came with a rig and clips, such as a Tripo job's animations.

The sheet shows the rig it read, the clips with their frames, rate, loop and root motion, all chosen, and the pack, a new one named after the model by default. A clip that came from the library is listed but never saved back, and keys the library has already are skipped. Only the skeleton and the clips' keys go in; the mesh and its textures stay where they are.

## The view

Search takes a clip, pack or tag. Grid or List changes the layout. The filters are Packs (All clips, each pack, Singles last), Tags and Source rig, each with its count over the whole library. **Root motion** and **Loops** keep only the clips that travel or loop. The stat strip shows how many clips, packs and singles there are and what they weigh on disk.

A card shows the clip's rest pose, its source rig, its name, its length ("38 f · 1.27 s") and whether it travels (root) or stays put (in place), and loops.

The side panel plays the selected clip on its own skeleton, looping, with its frame and how far the root travels; no model is needed to look at a clip. Under it: the pack and the file it came from, its tags, length and rate, root motion ("yes, 0.92 m to the left"), loop, source rig with its bone count, and what it stores.

The status bar reads "Animations at ~/Maquette/library/animations, N clips".

## Use on model

**Use on model** in the side panel, or **Add clip** > **Add from library** in [Motion](/docs/views/motion/), puts library clips on a rigged model of the open project in four steps. Nothing is baked until Review.

1. **Choose clips.** Pick the model, then the clips; search and the packs narrow the list. The side shows how many clips, frames, packs, root motion and loops are chosen, and what they add to the model: the motion only, as triangles and textures stay as they are.
2. **Bone map.** Each humanoid bone with the clip's bone and the model's. The model's side is read from the map saved for it, then from the bone names (Mixamo, Unreal and Rigify style names, and Tripo's own); pick another bone, or none to keep it at rest. The step opens on its own when a bone has no match and no map is saved; it never blocks. **Save this map** keeps it for the model's next retarget.
3. **Clip settings.** Per clip: its name in the model (snake_case, the pack's words dropped: Sword And Shield Idle reads `sword_idle`), the frames it keeps, loop, in place or root motion, speed and Pin the feet. They are this model's only and are never written back to the library.
4. **Review.** Blender bakes the clips headless onto a copy of the newest version. Each clip is good, to look at (a planted foot slides more than 1.5 cm) or failed (the hips leave by metres, so its keys are in the wrong units, or nothing maps). The selected clip plays on its own skeleton and on the baked copy side by side. **Open in Blender** opens the baked copy in Blender's window to look closer (the one time Maquette opens Blender with a window, and only on that click; nothing is saved back from there). **Bake N clips into vN** adds the copy as the model's next version through the usual checks, failed clips left out; the older version stays.

**Pin the feet and bake again**, on a clip flagged for a sliding foot, turns Pin the feet on for that clip and bakes again. A foot counts as planted while it is low (within 2.5 cm of its lowest point in the clip) and slow (it moves at most 8 cm along the ground from one frame to the next) for at least three frames; the review measures the slide with the same test. The first bake finds each foot's planted frames, then a second headless run holds the foot where it went down over each of them and bends the leg back to it with a two bone IK (thigh and shin, the knee bending the way it bends in the clip), easing back over three frames after. The foot keeps the clip's turn. The review then shows the slide before and after, for example "Feet pinned, the slide is down from 4.2 cm to 0.3 cm"; a foot the leg cannot reach still slides and stays to look at. The setting is kept with the clip's settings for this model.

How it bakes: the model's limbs are first turned to the clip's rest pose so both rigs start matched (a T pose clip on an A pose model), then each frame takes the world rotation of every clip bone against its rest and applies that change to the mapped bone. The hips move scaled by the model's hips height over the clip's. Fingers and unmapped bones stay at rest and follow their parents.

The side panel lists **Where it is used**: each model that baked the clip, with its project, version and name there. When an import replaces a clip with a newer take, those models show **Update**, which opens Use on model at Clip settings with the settings it was baked with; the new bake replaces the old clip in the model's next version.

## Mixamo clips

Mixamo clips may be used in your own games and sites, but not handed out as raw files. The library lives on this machine and is never shipped: Ship and Export never copy a raw clip, and a clip leaves the library only baked into a model.

## Where the files live

The clips are in `library/animations/` in the workspace root, by default `~/Maquette/library/animations`. Each clip is a folder `library/animations/<clip-id>/`:

- `clip.json`: the name, pack, tags, source rig with its bones and rest pose and its hips height, frames, rate, length, root motion, loop, size, the file it came from and the hash of its keys.
- `clip.glb`: the skeleton and the one animation, no mesh.
- `source.fbx`: the original file, only when Keep the FBX files was on.

What a project took from the library is in its own `animations.json`: per asset, the saved bone map, each clip baked in with its settings, revision and version, and the last review.

Agents use the same library through [`list_animations`](/docs/agents/mcp-tools/#list_animations), [`import_animations`](/docs/agents/mcp-tools/#import_animations), [`save_model_clips`](/docs/agents/mcp-tools/#save_model_clips), [`use_animations`](/docs/agents/mcp-tools/#use_animations) and [`get_retarget_review`](/docs/agents/mcp-tools/#get_retarget_review).
