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

**New recipe** picks the base clip from the library and names the recipe after it unless you type a name. Each recipe is one small file in the project, `recipes/<id>.recipe.json`, holding the base clip's name, frame count, length and hash, the blanks (what each project fills in, such as which prop the hand reaches) and the changes, on humanoid bone names. It holds no keys, mesh or skeleton, so it can be shared. On another computer the base clip is found in its library by name and frame count; a clip with other keys only warns. A blank filled with a prop the project no longer has reads as open, with a warning on the recipe's line; the file keeps the prop, so restoring the asset fills the blank again. To empty it for good, open the blank and choose **Leave open** before you save.

A recipe on a Mixamo clip is marked **Mixamo**: the recipe file can be shared, but what it bakes counts as Mixamo, so export packs leave it out and it never goes into the clip library.

**Edit** opens a recipe. Its changes are listed in the order they are laid over the clip; open one to set its kind, its bone (a reach moves a hand or foot, a look at the head or neck, a wave an arm, a nudge any humanoid bone), the blank it aims at, its weight and where it plays, by dragging the range over the clip or typing the frames. A nudge adds degrees and metres to its bone, a wave sets how many waves and how far each swings. Blanks are filled with a prop of the project or a point in metres from where the model stands: X to its left, Y up, Z the way it faces. Anything the recipe would refuse is said next to it, and Save waits until it is fixed. Agents fill in changes and blanks over MCP too ([`update_recipe`](/docs/agents/mcp-tools/#update_recipe)).

Edits stay until you save them, also when you leave the tab or the Motion view. Leaving the recipe, opening another one, switching project or quitting with unsaved edits asks first: save, stay, or leave without saving. When an agent changes the recipe while it is open, the editor shows the new version if you have nothing unsaved, and otherwise merges it with your edits change by change; only when both sides changed the same change or blank does it ask **Keep mine** or **Use theirs**, and nothing saves until you answer. Save also reads the file first and saves only against the version it read: when an agent writes in between, nothing is written, its change is merged with yours the same way (asking only on a clash) and you save again. Quitting while a save runs waits for it.

The preview plays the base clip on the project's rigged model, picked in the list, through its bone map, with the changes laid over it roughly: a reach bends its arm or leg toward the target, a look at turns the head, a nudge eases its turn or move in and out, a wave swings on a sine. Small markers show each filled blank: a point where it says, a prop in a spot in front of the model, since a recipe does not say where a prop stands. Which way is in front is read from the model's shoulders (or hips), so a model that faces another way than +Z still reaches, looks and waves the right way; the rough retarget under it still lines limbs up in world space, so a model facing away from the clip's way plays it turned. It says it is rough, because the bake solves the same changes exactly. Under it, play, pause and scrub the clip, with each change drawn over the part of the clip it covers. At each reach's end it warns when the hand or foot stays more than 2 cm from its target, or the elbow or knee bends the other way than the clip has it.

**Apply** bakes the recipe onto the project's rigged model, the one picked in the list (or the first), in headless Blender: the base clip goes on through the same retarget as the Animations library, then the changes are solved over it exactly, frame by frame, into one new clip named after the recipe. A reach is solved by IK to its target, a look at turns the shortest way and holds within 80 degrees, a nudge turns and moves about the model's own axes, and a nudge and a wave ease in and out as the preview does. A prop target sits where the scene layout places the prop when both it and the model are placed, and otherwise in the preview's spot in front of the model. Apply waits while a blank is open or the recipe has no changes. In the editor, unsaved edits are saved first (**Save and apply**).

The bake runs in the background with its progress under the recipe, and **Cancel** stops Blender part way. A bake still running when the app quits reads as stopped when it opens again, so bake it again. When it is done it shows the new clip and its numbers: how far the planted feet slide (more than 1.5 cm counts, next to the base clip's own slide), how far the rig stretches past a healthy skin, each reach's gap to its target at its end (reached within 2 cm) and the first frame its elbow or knee flips, and how long it took. The baked model is staged in the workspace cache, one per recipe, and a new bake replaces it; it is not added to the model yet. Review and add as a version comes next. A bake of a Mixamo recipe is marked **Mixamo** and never goes into the clip library or an export pack. Agents bake over MCP too ([`bake_recipe`](/docs/agents/mcp-tools/#bake_recipe), [`get_recipe_bake`](/docs/agents/mcp-tools/#get_recipe_bake)).
