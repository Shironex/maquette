---
title: Models
description: Keep finished models in one place outside every project, and copy them into any project, with their real size, fix-ups and material tweaks.
sidebar:
  label: Models
---

The models library keeps models you want to use again. It lives in the workspace, outside every project, and shows in the rail as Models under Libraries, with the number of models after its name, for website and game projects. Animation clips have a library of their own next to it, [Animations](/docs/views/animations/). It copies a model into a project and offers updates; it does not link. A project always owns its copy, and nothing ever changes a project silently.

## Add to library

There are four ways in: **Add to library...** in a Board card's menu (it reads No model yet when the asset has no model), the Library section of the [Inspector](/docs/views/inspector/), **Add from a project** in the Library view, where you pick the project and the asset, and **Add a whole project**, which adds every accepted model of a project in one go (see below).

You can set the name and comma separated tags. Tags are lowercased, with at most 16 per model and 32 characters each. Maquette copies the newest model, its picture, its refs, its real size, its fix-ups and its material tweaks. The asset then remembers which entry and version it came from.

Adding the same model again (the same sha256 hash) adds nothing. A newer model of an asset that is already in the library adds the next version to its entry.

## The view

The top bar reads Models with how many the library holds. Grid, List or Look-alikes changes what the middle shows. The filter column has:

- **Collections:** All models, then each collection with its count, and **New collection** at the foot. A collection is a named set of models; a model can be in several. A new collection starts with the selected models, if any.
- **Tags:** each tag with its count, and **untagged** for the models with none.
- **Made with** (Tripo, Meshy, By hand, Built or Imported), **Format** (.glb or .obj, shown when both exist) and **Used in** (each project, or Not used).

The stat strip shows Models with how many are shared, Credits saved by reuse and Updates waiting. Under it a line names what is shown (All models, Models found or the collection) with the count, then a button for the open look-alike groups, one for the untagged models, and the sort: Newest first, Most used, By name, Most triangles or Fewest triangles.

### Search

Search takes plain words, which match a model's name, tags, the projects that use it and what made it. A few words narrow further, and they combine:

| Word           | Finds                                                                       |
| -------------- | --------------------------------------------------------------------------- |
| `tag:lights`   | models with a tag holding "lights"                                          |
| `in:frostveil` | models a project with that name or id uses                                  |
| `by:tripo`     | models whose newest version Tripo made (or meshy, built, by-hand, imported) |
| `<5k`, `>20k`  | the newest version's triangles, under or over                               |
| `tris<5000`    | the same, spelled out                                                       |

### Cards

A card shows the picture, the name, the latest version, source and triangles ("v3 · Built · 412k tris"; from 10k up the count is in whole thousands), the tags (or untagged), how many projects use it and its credits: "30 credits", "no credits, built" or "30 saved by reuse". A card in a look-alike or same name group carries a **look-alike** or **same name** flag.

A click on a card's picture, or on the big picture in the side panel, opens the newest version in a 3D quick view: the Inbox's preview with orbit, zoom and its modes, the version, triangles and size. A click anywhere else on the card selects it. Open in Inspector shows the open project's copy of the model; a model the project has not added yet says so instead. An `.obj` model has no 3D preview, so its picture only selects it.

The grid and the list render only the models in view, so a library of hundreds scrolls as fast as one of ten.

The side panel shows the selected model:

- **Use in project:** pick the project, and a version when there are several. **Add, no credits** copies it.
- **Tags:** each tag comes off with its x; type one and press Enter to add it.
- **Versions:** each version with its source, credits, when it was added and how many copies use it.
- **Where it is used:** every asset copied from this model, marked current or with an Update to vN button.
- **Credits:** what it cost to make, and what reusing it saved.

The status bar reads "Library at ~/Maquette/library, N models".

## Select many

Every library action takes a selection, so nothing has to be done one model at a time. Hover a card and tick the box on its picture, or Cmd or Ctrl and click the card. Shift picks every model between the last one and this one. While a selection is under way any click on a card adds or drops it, and the side panel steps aside. Escape or the x clears it.

A bar over the models then reads how many are selected, with **Select all** for every model in view, and:

- **Tag:** a popover with a field. Type a tag and press Enter to put it on every selected model; the tags it matches show how many of the selection carry them. Under **On all** each tag on every selected model comes off with its x; under **On some** a click puts that tag on the rest.
- **Add to collection:** pick a collection; the models already in it are left as they are. New collection in the filter column makes one that starts with the selection.
- **Use in project:** pick a project, and each model's newest version is copied in, at no credits.
- **Remove:** takes the models out of the library. Copies in projects stay as they are. The notice has an Undo that brings them back.

## Look-alikes

Look-alikes finds two kinds of pairs:

- **Same look:** two models whose pictures look the same, compared the way the Inbox matches downloads: outline, colour and box ratio, together. A pair at or above the threshold (0.90 by default) is shown with the three scores. **Merge as versions** makes the newer model the next version (or versions) of the older one; a project that used the newer model keeps the model it has and now points at the merged entry. **Keep both** and **Not alike** leave both and hide the pair.
- **Same name:** two different models under one name ("Stone well" and "Stone well 2"). Give each a name that says what it is and **Rename both**, or **Keep as is**.

The first two pairs open whole; the rest wait on one line with **Review**. Recent merges are listed with the time and an **Undo** that puts both models back as they were, origins included.

The side holds the threshold, **Check when models are added** (adding a whole project asks about a model that looks like one already in the library), **Copy tags on merge** (the merged model keeps the tags of both), the last scan and **Scan again**. Models with no picture are skipped by the picture match and named in a warning; names still pair. The scan reads only, and runs again when the library changes size.

## Add a whole project

**Add a whole project** opens a sheet for the open project (pick another at the top). It adds every accepted model, the newest version of each; older versions stay in the project. Names and tags are settled before anything is copied:

- **Project tag:** the project's name as a tag, on every model, with a switch to leave it off.
- **Collection:** none, an existing one, or a new one by name.
- **Tags by kind:** comma separated tags for the props, characters and scenes.
- **Already in the library:** a model that is the same file as one in the library is skipped without asking. One that shares a name with a library model, or looks like one when Check when models are added is on, gets a choice: **Add as a new version** of that model, **Keep both** (a kept name clash gets a number, such as "Stone well 3"), or **Skip**. It starts on a version when the two look alike (0.90 or more) and on keep both when they do not.
- **Could not be read:** a missing file, or a `.glb` cut short, stays in the project; add it after a redo.

**Add N models** copies them one after another with a progress bar. **Stop after this one** finishes the model being copied, then stops. Tags and the collection are set once the copies are done. It spends no credits.

## Use in project

Use in project copies the chosen version (the newest by default) into the project as a new asset at v1, at the Blender stage and done. It brings the picture, refs, real size, fix-ups and material tweaks, and records its origin. A name that is taken gets a number, such as "Desk lamp 2". It spends no credits.

The project decides what it takes:

- **Website:** the scene is built from `.glb` models, so an `.obj` entry is refused, with the reason.
- **Game:** takes `.glb` or `.obj`. The next run's Game export writes the model into the game's models folder in the game's format. If the game wants `.glb` and the model is `.obj`, that run writes it as `.glb`. Nothing is converted when you add it.

## Updates

When an entry gets a newer version, every asset copied from an older one shows Update to vN on its Board card, **Update to library vN...** in the card menu, **Update available** in the Inspector's Library section, and in the Library side panel.

Update opens a dialog with the plan. It lists what carries over (real size, fix-ups, skip bake and material tweaks) and what no longer applies, with the reason:

- A Decimate fix-up is dropped when the new model already has that many triangles or fewer.
- Shrink textures is dropped when the new model has no textures, or none larger than the limit.
- Material tweaks for a material name the new model no longer has are dropped.

**Update, no credits** adds the library version as the asset's next version. Older versions stay. **Keep vN** closes the dialog and changes nothing.

## Where the files live

The library is `library/` in the workspace root, by default `~/Maquette/library`. Each entry is a folder `library/<entry-id>/`:

- `entry.json`: the name, kind, tags, refs and versions.
- `models/`: one file per version, `<key>_v<n>.glb`, or a folder for an `.obj` with its files.
- `thumbs/`: each version's picture.
- `refs/`: the entry's reference images.

Next to the entries, `library/collections.json` holds the collections and `library/look-alikes.json` the look-alike settings and the pairs you decided. A removed or merged entry moves to `library/.removed/<entry-id>/`, where restore finds it; a merged one keeps what undoing the merge needs.

Each version records the model file, when it was added, its stats (triangles, bytes, textures), its source, the credits it cost when known, its hash, the project, asset and version it came from, its picture, and a snapshot of the asset's real size, fix-ups and material tweaks. Agents use the Library through the [MCP tools](/docs/agents/mcp-tools/).
