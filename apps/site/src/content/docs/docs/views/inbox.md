---
title: Inbox
description: Models from Tripo, Meshy or your own files arrive here. Maquette checks them, offers fix-ups and moves the keepers into a project.
sidebar:
  label: Inbox
---

Maquette watches the workspace `inbox/` folder for `.glb` and `.gltf` files. Each file is read in Rust: triangles, meshes, materials, textures and their sizes, skins and joints, animations.

## Checks

The checks compare that with your budgets: triangles, texture size, a Mixamo rig for characters, PBR maps, separate objects merged into one mesh, an origin away from the bottom centre, several objects that could be one, and hair or tail vertices weighted to finger bones. Two of them are heuristics: merged objects and weighted to finger bones. Draco or meshopt compressed geometry skips both.

A check that a fix-up answers offers it as a button: Decimate to 60k, Shrink textures to 2048 px, Remove loose parts, Origin to bottom centre or Merge objects.

## Preview

The preview stands every model in the same studio: a graphite backdrop, a soft floor with a contact shadow and warm and cool lights. Pick Shaded, wireframe over clay, a numbered UV checker, or the skeleton drawn over a ghost of the mesh. It reads meshopt geometry and KTX2 textures.

## Real size and fix-ups

Each asset can have a real size and a list of fix-ups, both optional. A size is one side of the real object (height, width, depth or the longest side) in cm or m. The fix-ups run in a fixed order: Apply transforms, Remove loose parts, Merge objects, Decimate, Shrink textures and Origin to bottom centre. Nothing touches the inbox file or the asset's versions: the [Build scene](/docs/pipeline/build-scene/) step applies them headless on a copy.

You can also skip the bake for an asset and pad its texture edges. A model accepted from the Inbox skips the bake and pads by default, since provider scans cut their textures into many small islands.

## Accept and redo

Accept to Blender moves the file into the project's `models/` folder as a new asset version. By default it goes to the asset the model was sent for, or the one a download was attached to. A model accepted for an asset with a By hand send still waiting also closes that send, as done with the note "Model added to the asset, no longer waiting"; a Tripo or Meshy send of the asset runs on. Redo sends an inbox model's asset again, prefilled from its send; the earlier model stays in the inbox so you can compare and accept one.

## Downloads from By hand sends

While a By hand send waits, the Downloads watch (Settings > 3D providers) looks at new `.glb`, `.fbx` and `.zip` files. A file named for its send (`maq_<asset>_v<n>_<provider>`) goes into the inbox by itself. Anything else waits in **New downloads**, opened from its notice:

- **Needs a look**: the name is only close to a waiting send's (a site's batch export such as `lantern post 3d model.glb`), or it is the only send waiting. The card shows the model's thumbnail, rendered headless, next to the ref the send went out with, the file name and why it matched. **Attach to Lantern post** takes it, **Pick another asset** chooses a different one, and **Not this one** leaves the file alone and never asks about it again.
- **Which asset is this?**: files it could not place. **Attach to an asset** opens a searchable list of the project's assets, the ones a By hand send waits on first, each with its ref picture and state. Type to search, use the arrows and press Enter; the list moves on to the next file, so a batch goes quickly. Esc closes it.

Attaching sends the model to the inbox for that asset, as a file named for its send would go: for the asset's newest waiting By hand send, or aimed at the asset when none waits. The usual checks run, and the real size and fix-ups draft is there as for any inbox model. Accept goes to that asset by default and closes its By hand sends still waiting. The downloaded file stays where it is.
