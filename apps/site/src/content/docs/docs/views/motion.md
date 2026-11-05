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

The Recipes tab lists the project's animation recipes, in website and game projects alike. A recipe is a clip from the [Animations](/docs/views/animations/) library with changes laid over it: a hand or foot reaching a prop or a point, a nudge to one bone's pose, the head looking at something, or a wave. Applying a recipe bakes one clip into the next version of a model, never an extra clip on the version you have.

**New recipe** picks the base clip from the library and names the recipe after it unless you type a name. Each recipe is one small file in the project, `recipes/<id>.recipe.json`, holding the base clip's name, frame count, length and hash, the blanks (what each project fills in, such as which prop the hand reaches) and the changes, on humanoid bone names. It holds no keys, mesh or skeleton, so it can be shared. On another computer the base clip is found in its library by name and frame count; a clip with other keys only warns.

A recipe on a Mixamo clip is marked **Mixamo**: the recipe file can be shared, but what it bakes counts as Mixamo, so export packs leave it out and it never goes into the clip library.

For now the tab lists and makes recipes, and agents fill in their changes and blanks over MCP ([`update_recipe`](/docs/agents/mcp-tools/#update_recipe)). Editing changes in the app, the preview and **Apply** come in later updates; Apply says so until then.
