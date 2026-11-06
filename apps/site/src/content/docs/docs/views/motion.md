---
title: Motion
description: Read the clips a model carries, check its rig, pose the parts of a prop as named states and keep animation recipes.
sidebar:
  label: Motion
---

Motion lists the project's models that move: a rig, animation clips or object states. The ones that do not move yet sit under No motion yet.

## Clips

Every clip is read from the GLB as it is: name, frames, key rate, keys, channels, keyframe bytes, how far the root travels and whether it loops. Pick a model and a clip to play it in the studio with the skeleton and the path its root walks. Each clip has a ship switch: one switched off stays in the model but is left out of the site from the next Build scene. In a [game project](/docs/start/game-projects/) the switch says the game file instead: it saves at once, and the next export leaves that clip out of the model's game file, with the source version untouched.

Clips baked from the library sit under **From the library**, apart from the clips in the model's own file. The stage then names the clip they came from, the timeline says when one was trimmed from a longer library clip, and after a bake a banner says what the new version added, with **Compare with vN** opening the two versions in the [Inspector](/docs/views/inspector/). A clip that failed review is listed there as left out.

**Add clip** > **Add from library** puts library clips on the selected rigged model: see [Use on model](/docs/views/animations/#use-on-model). A model with a rig and no clips offers the same from **Retarget a clip**.

**Add clip** > **Save clips to library** copies a rigged model's clips into the [Animations](/docs/views/animations/) library as a pack, skeleton only, so any other project's model can take them. Clips that came from the library are never saved back.

## Rig check

The rig check shows the bone count, whether the rig maps onto the humanoid (Mixamo) skeleton, the most bones one vertex weighs on against the web's four, and which humanoid bones are missing. Its last line is the model's VRM readiness, as the [Inbox](/docs/views/inbox/#vrm-readiness) reads it. **Map bones** opens Use on model at its bone map, where each humanoid bone can be given another rig bone or left at rest. On a character, **Open Avatar** opens its VRM export in [Avatar](/docs/views/avatar/).

## Budget

Motion has no budget of its own. Clips are bytes inside the models, so they count in the scene budget in the top bar. Over budget, the view marks the clips that would have to stay off, heaviest first.

## States

A prop's states are poses of its parts. The first state is the model at rest. Every other state moves named parts by an offset in metres and degrees. Between two states a transition has a duration, delay, easing and stagger. Triggers say what starts a move on the site: arriving at a camera stop, a scroll range, a click or a hover. Up to 16 states and 32 triggers per asset.

Save motion writes the states and clip choices into `layout.json`, next to the camera stops. The site plays the stop and scroll triggers as the page scrolls; click and hover do not play there yet. The [Timeline](/docs/views/timeline/) places the stop and scroll triggers along the page.

## Recipes

The Recipes tab lists the project's animation recipes, in website and game projects alike. A recipe is a base clip from the [Animations](/docs/views/animations/) library, or the model's rest pose, with changes laid over it: a hand or foot reaching a prop or a point, a nudge, the head looking at something, or a wave. Applying one bakes a clip into the next version of a model.

The tab has:

- **New recipe**, with a clip picker that also offers **Rest pose**, and **Import** in the header. Each row shows a **Mixamo** badge, open blanks and a warning when its base clip is not in the library.
- **Edit**: changes and blanks on a timeline range, with checks next to the fields and unsaved edits kept until you decide.
- A rough preview of the changes over the base clip on the selected model.
- **Apply** (or **Save and apply**): the exact bake in headless Blender, with progress and **Cancel**. When it is done, **Review** plays it next to the base clip with each number against its limit, and **Add as vN** adds it as the model's next version.
- A row menu with **Save to library** and **Export**, and **Library has rN** on a recipe whose library recipe has a newer revision.

How each part works, what the numbers mean and how agents use recipes is in the guide: [Animation recipes](/docs/guides/animation-recipes/). Agents work with recipes over MCP, with [`update_recipe`](/docs/agents/mcp-tools/#update_recipe), [`bake_recipe`](/docs/agents/mcp-tools/#bake_recipe), [`get_recipe_bake`](/docs/agents/mcp-tools/#get_recipe_bake) and [`add_recipe_bake`](/docs/agents/mcp-tools/#add_recipe_bake).
