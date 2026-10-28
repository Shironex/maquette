---
title: Models
description: Keep finished models in one place outside every project, and copy them into any project, with their real size, fix-ups and material tweaks.
sidebar:
  label: Models
---

The models library keeps models you want to use again. It lives in the workspace, outside every project, and shows in the rail as Models under Libraries, with the number of models after its name, for website and game projects. Animation clips have a library of their own next to it, [Animations](/docs/views/animations/). It copies a model into a project and offers updates; it does not link. A project always owns its copy, and nothing ever changes a project silently.

## Add to library

There are three ways in: **Add to library...** in a Board card's menu (it reads No model yet when the asset has no model), the Library section of the [Inspector](/docs/views/inspector/), and **Add from a project** in the Library view, where you pick the project and the asset.

You can set the name and comma separated tags. Tags are lowercased, with at most 16 per model and 32 characters each. Maquette copies the newest model, its picture, its refs, its real size, its fix-ups and its material tweaks. The asset then remembers which entry and version it came from.

Adding the same model again (the same sha256 hash) adds nothing. A newer model of an asset that is already in the library adds the next version to its entry.

## The view

Search takes a model, tag or project name. Grid or List changes the layout. The filters are Tags, Made with (Tripo, Meshy, By hand, Built or Imported), Format (.glb or .obj, shown when both exist) and Used in (each project, or Not used). The stat strip shows Models with how many are shared, Credits saved by reuse and Updates waiting. Sort by Recent, Most used or Name.

A card shows the picture, the name, the latest version and source ("v3 · Built"), the tags, how many projects use it and its credits: "30 credits", "no credits, built" or "30 saved by reuse".

The side panel shows the selected model:

- **Use in project:** pick the project, and a version when there are several. **Add, no credits** copies it.
- **Versions:** each version with its source, credits, when it was added and how many copies use it.
- **Where it is used:** every asset copied from this model, marked current or with an Update to vN button.
- **Credits:** what it cost to make, and what reusing it saved.

The status bar reads "Library at ~/Maquette/library, N models".

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

Each version records the model file, when it was added, its stats (triangles, bytes, textures), its source, the credits it cost when known, its hash, the project, asset and version it came from, its picture, and a snapshot of the asset's real size, fix-ups and material tweaks. Agents use the Library through the [MCP tools](/docs/agents/mcp-tools/).
