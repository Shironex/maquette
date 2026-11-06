---
title: Animation recipes
description: Change a clip without touching its keys. A recipe is a base clip or the rest pose plus changes (reach, nudge, look at, wave). Edit it, preview it, bake it exactly in Blender, review the numbers and add it as the model's next version.
sidebar:
  label: Animation recipes
---

A recipe changes an animation without redrawing it. It starts from a base clip (or from the model's rest pose) and lays a few changes over it: a hand reaching a prop, a bone nudged, the head following a point, a wave. Maquette bakes the result into one new clip on your model.

This guide covers the whole path. The [Motion view](/docs/views/motion/#recipes) and the [Animations library](/docs/views/animations/#recipes) show where each part lives in the app.

## What a recipe is

A recipe is one small file in the project, `recipes/<id>.recipe.json`. It holds:

- the base: a clip's name, frame count, length, hash and whether it came from a Mixamo rig, or the rest pose (see [Rest pose](#rest-pose))
- the blanks: what each project fills in, such as which prop a hand reaches
- the changes, on humanoid bone names

It never holds keys, frames, a mesh, a skeleton or a render. So a recipe is small, it can be shared, and it works on any humanoid model. On another computer the base clip is found again in the clip library by its name and frame count. A clip with other keys only warns.

Recipes live in two places: in a project, where you edit and bake them, and in the [recipe library](#the-recipe-library), where you keep them for other projects.

## Make a recipe

In the Motion view, open the Recipes tab and choose **New recipe**. Pick a base clip from the library, or **Rest pose**. The recipe is named after the clip unless you type a name. You can also start from a [starter](#starters) or from the library, or [import](#import-and-export) a file.

Website and game projects both take recipes.

## The editor

**Edit** opens a recipe. Its changes are listed in the order they are laid over the clip.

- **Changes.** Each change has a kind, a bone, the blank it aims at (when its kind needs one), a weight from 0 to 1, and a start and end on a timeline range. Drag the range over the clip or type the frames.
- **Blanks.** A blank is a target left open on purpose. Fill it with a prop of the project or with a point.
- **Inline checks.** Anything the recipe would refuse is said next to the field, and Save waits until it is fixed.
- **Unsaved edits.** Edits stay until you save them, also when you leave the tab or the view. Leaving the recipe, opening another one, switching project or quitting asks first: save, stay, or leave without saving.

When an agent changes the recipe while it is open, the editor shows the new version if you have nothing unsaved. Otherwise it merges the two change by change. Only when both sides changed the same change or blank does it ask **Keep mine** or **Use theirs**. Save also reads the file first and saves only against the version it read. If something wrote in between, nothing is overwritten: it merges and you save again.

## Blanks

A blank holds a target for a reach or a look at. It is either:

- a **prop**: an asset of the project. The target is the prop's middle where the scene layout places both the prop and the model, and otherwise a spot in front of the model.
- a **point**: x, y and z in metres, in the model's own frame. X is to its left, Y is up, Z is ahead of it.

A recipe with an open blank cannot bake. A prop blank whose asset the project no longer has reads as open, with a warning. The file keeps the prop, so restoring the asset fills the blank again. To empty it for good, open the blank and choose **Leave open** before you save.

## The four kinds of change

| Kind        | Bones                                              | What it does                                                                                |
| ----------- | -------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| **Reach**   | a hand or a foot                                   | Moves the hand or foot to a blank's target with IK, easing in and out.                      |
| **Nudge**   | any humanoid bone                                  | Adds degrees and metres to the bone's pose, about the model's own axes, eased in and out.   |
| **Look at** | head or neck                                       | Turns the bone toward a blank's target the shortest way. The turn is clamped to 80 degrees. |
| **Wave**    | an upper arm, a lower arm or a hand, left or right | Swings the arm on a sine: you set how many waves and how far each swings, in degrees.       |

A limb held straight is pre-bent 10 degrees before the bake solves a reach, so IK can bend it.

## Rough preview and exact bake

The preview plays the base clip on the project's rigged model with the changes laid over it roughly: a reach bends its limb toward the target, a look at turns the head, and small markers show each filled blank. It is labelled rough because the bake solves the same changes exactly. Under it you can play, pause and scrub, with each change drawn over the part of the clip it covers.

Which way is front is read from the model's shoulders (or hips), so a model that faces another way than +Z still reaches, looks and waves the right way. The rough retarget under the preview still lines limbs up in world space, so a model facing away from the clip's way plays it turned.

**Apply** bakes the recipe in headless Blender, frame by frame, into a staged model with one new clip named after the recipe. Nothing is added to the project yet. Apply waits while a blank is open or the recipe has no changes. In the editor, **Save and apply** saves your edits first. The bake runs in the background, **Cancel** stops Blender part way, and a bake the app closed on reads as stopped: bake it again.

## The review numbers

**Review** on a finished bake plays the baked clip next to the base clip, with one play button for both. Each number is held against a limit and marked as pass, worth a look (warn) or not measured.

| Number               | Limit                                                          |
| -------------------- | -------------------------------------------------------------- |
| **Hand or foot gap** | Each reach ends within 2 cm of its target.                     |
| **Elbow or knee**    | It does not bend the other way than the clip has it (no flip). |
| **Foot slide**       | Planted feet slide up to 1.5 cm more than the base clip's own. |
| **Rig stretch**      | The rig stretches up to 5 mm more than the base clip's own.    |

A warn never stops an add. It tells you where to look.

What the numbers cannot tell you:

- They measure only what is in the table. They do not judge whether the motion looks good, and they do not check whether the arm passes through the body or the prop.
- The hand or foot gap is measured at each reach's end, not along the way. Rig stretch is the worst of 10 sampled frames and the reach ends, so a short spike between samples can be missed.
- Foot slide and rig stretch are held against the base clip's own numbers, so a clip that already slides or stretches is not blamed for it.
- A bake of a rest pose recipe is measured against the rest pose, which stands still. See [Rest pose](#rest-pose).

Always play the review before you add.

## Add as vN

**Add as vN** in the bar at the bottom adds the staged model as the model's next version. It goes the way the [Rig doctor](/docs/views/rig-doctor/) adds its draft: into the Inbox for the asset, then accepted, with its checks and thumbnail. The new version records which recipe and recipe version made it, and the staged model is cleared.

- A bake that is out of date (the recipe changed, or the model has another newest version) is never added. The review says why and offers **Bake again**.
- Add takes only the bake you looked at. If the recipe was baked again since, it is refused and you look at the new one first. While an add runs, the recipe cannot bake.
- **Clip names.** The clip is named after the recipe. Baking again replaces every clip this recipe made on the model, whatever it was named, so reruns never pile up `_3` and `_4`. Another clip with the same name from anywhere else stays, and the new one takes `_2`. The bake says which happened.

## Mixamo

A recipe file on a Mixamo clip can be shared, but what it bakes counts as Mixamo's. The recipe is marked **Mixamo**, and:

- export packs leave the model's baked clips out, also in later versions made by Blender
- the baked clip is never saved to the clip library
- the project's `recipe-bakes.json` records it, so the flag survives a Blender step that drops the mark in the file

A bake of a rest pose recipe is never Mixamo's.

## The recipe library

The workspace has a recipe library next to the clip library, in `library/recipes/<id>/`: an `entry.json` and one `rN.recipe.json` per revision. The Animations view lists it under **Show > Recipes**.

- **Revisions never change.** Saving always adds the next one, and a revision number is never used twice. Remove rN or Remove recipe asks first, and removed files move aside to `library/recipes/.removed/`. The last revision goes only with the whole recipe.
- **Use in project** copies a revision into a project as a new recipe that remembers the library recipe and revision. Point blanks keep their points. A prop blank whose asset the project lacks opens, with a note. A missing base clip still makes the recipe, and says so.
- **Library has rN** shows on a project recipe when the library has a newer revision. **What changed** lists it in plain words, and **Use rN** copies it in as a new recipe. Yours stays as it is. A library recipe that was removed reads as gone, even if a later recipe took its id.

### Save back

**Save to library** in a recipe's **...** menu opens a sheet. Choose **New revision of** the recipe it came from, or **New library recipe**. The sheet lists in plain words what changes against the newest revision. Which prop fills a blank belongs to each project, so it does not count. **Save as rN** writes the next revision.

- A recipe that holds what the newest revision holds has nothing to save.
- If the library recipe changed since the sheet opened, the save is refused with nothing written (`moved`) and the sheet reads it again for you to look.

## Import and export

**Export** saves a recipe as one `.recipe.json` wherever you pick. The file holds the base, the blanks and the changes, and nothing else: no keys, mesh, skeleton, render, path or library link. **Import** reads one into the project as a new recipe, and **Import recipe** in the Animations view adds one to the library at r1.

The file is checked whole before anything is written:

- at most 256 KB, checked before it is read
- version 1
- at most 32 changes and 16 blanks
- every check a recipe passes

A taken id gets a fresh one, and nothing in the file becomes a path. Notes under the header say what to know afterwards: a base clip that is missing, prop blanks that opened, a fresh id.

Agents import into a project only, from files inside the workspace named `*.recipe.json`. They export only into new files under `exports/recipes/` in the workspace. You can import and export anywhere.

## Starters

Five recipes are built into the app. They are read only and never written to the library unless you copy one.

| Starter                 | What it does                                                         | Base      |
| ----------------------- | -------------------------------------------------------------------- | --------- |
| **Wave hello**          | Raises the right upper arm and waves the right lower arm. 90 frames. | Rest pose |
| **Look at a point**     | The head looks at a point blank.                                     | An idle   |
| **Reach for a prop**    | The right hand reaches an open prop blank.                           | An idle   |
| **Lean in**             | Nudges the spine and head.                                           | An idle   |
| **Point while walking** | The right hand reaches a point blank.                                | A walk    |

A starter names its base clip by name only, never by hash, keys or frames. The names are tried in order: for an idle, Idle, Breathing Idle, Standing Idle, Happy Idle. For a walk, Walking, Walk. Used in a project (or copied to the library), a starter takes the first clip of the clip library with one of those names, whatever its frame count, and stores that clip's base.

With none of those clips in the library, the recipe keeps the first name only and reads as missing its base, like any recipe whose clip is not in the library. Once a clip of that name is imported, it matches by name alone.

In the app, **Show > Recipes** in the Animations view has a **Starters** group with **Use in project** and **Save to library**. Save to library makes an editable copy: a new library recipe at r1, its revision noted as imported from "Starter: name".

## Rest pose

A recipe can run on the model's rest pose with no base clip. Choose **Rest pose** in New recipe's clip picker. The file stays version 1: the base is marked `rest`, named "Rest pose", with an empty hash, and 60 frames at 30 fps by default (2 to 600 frames). Older files read as before.

- The rough preview plays the changes over the rest pose.
- The bake holds the rest pose for the recipe's frames and lays the changes over it.
- Review numbers work the same way. Foot slide is measured against the rest pose, which stands still, and rig stretch against the rest pose's own.
- The bake names no base clip, so a change in the clip library never makes it out of date. It is never Mixamo's.

## Agents

Agents use the [recipe tools](/docs/agents/mcp-tools/) and the `maquette-animation-recipe` [skill](/docs/agents/skills/). The usual workflow:

1. `list_library_recipes` shows the starters (ids start with `starter:`) and the library recipes. `use_recipe` copies one into a project. Or `create_recipe` makes one on a clip, or on the rest pose.
2. `get_recipe` reads the recipe and its `version`. `update_recipe` sends the changes and blanks with that version as `expected`. The lists replace the whole list, so send every entry you keep. Fill the blanks.
3. `bake_recipe` answers at once. `get_recipe_bake` is polled for the state and the review numbers.
4. `add_recipe_bake` asks you to add the bake as a version.
5. `propose_recipe_save` asks you to save the recipe to the library.

What waits for a person:

- adding a bake as a version: you review it in the Motion view and add it there
- saving to the library, which includes copying a starter there (an agent uses it in a project, then proposes the save)
- removing library recipes or revisions
- the app's Import to library, and exporting outside `exports/recipes`

Next: [Motion](/docs/views/motion/), [Animations library](/docs/views/animations/), [MCP tools](/docs/agents/mcp-tools/).
