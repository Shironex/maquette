---
title: Build scene
description: Places each model in one Blender scene from the layout, with its real size and fix-ups applied.
sidebar:
  label: Build scene
---

Step 1 of 6 in a new project. Kind: `blender`. Script: `tools/build_scene.py`.

## What it does

It opens `blender/scene.blend` when there is one, else an empty scene with a sun. It imports each `models/*.glb` into its own collection under "Models", gives each model its fix-ups and real size from `prepare.json` through `fixups.py`, places them from the scene layout in `layout.json`, and saves `blender/scene_built.blend`.

- Models without a placement that overlap are laid out side by side.
- The layout's desk becomes a slab under the models, so they shadow it in the bake.
- The world is a mid grey (0.18), so the bake has a soft fill.
- Models set to skip the bake are tagged for the bake and the export.
- Camera stops become cameras in a "Stops" collection, unless the base blend has stop cameras of its own.
- Rigged models keep their armature, skin and clips. Clips the Motion view keeps off the site are left out.

## Inputs and outputs

**Inputs:** `blender/scene.blend`, `models/*.glb`, `models/*.gltf`, `layout.json`, `prepare.json` and `tools/fixups.py`. Saving a layout or a size reruns the step.

**Output:** `blender/scene_built.blend`.

## Fix-ups and padding

Fix-ups replay on every new version of an asset. Before Decimate and Shrink textures, the step can pad texture edges: it fills the texture around each UV island with the island's own colours, so shrunk or mipmapped textures show no light or dark seams. On a 626k triangle scan it adds about 9 s.
