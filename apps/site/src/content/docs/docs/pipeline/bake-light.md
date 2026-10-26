---
title: Bake light
description: Bakes Cycles light into one lightmap per object, on the CPU, then denoises each map.
sidebar:
  label: Bake light
---

Step 2 of 6 in a new project. Kind: `blender`. Script: `tools/bake_light.py`.

## What it does

It gives every mesh outside "Studio" a second UV named `lightmap` (Smart UV Project when it is missing) and bakes Cycles diffuse light, direct and indirect, into one `blender/bake/<object>_light.png` per object. It then fills the gaps between UV islands and denoises each map with OpenImageDenoise. The whole scene bakes together, so every model shadows the others and the desk.

Each map stores light divided by a scale of at least 1. `bake.json` lists every object's file, size and scale. The step saves `blender/scene_baked.blend`.

## Settings

The Baked light panel in the Pipeline view sets these. They are the step's `--size`, `--samples` and `--denoise` args, so editing the args by hand is the same thing, and a change reruns the bake.

| Setting       | Range          | New projects |
| ------------- | -------------- | ------------ |
| Lightmap size | 256 to 4096 px | 1024         |
| Samples       | 1 to 4096      | 32           |
| Denoise       | on or off      | on           |

Measured on a study room (Apple M3, CPU): 512 px at 16 samples without the denoiser took 20 s for 33 meshes and left blotches on flat surfaces. With 9 scans skipping the bake, 1024 px at 32 samples took 143 s for 21 meshes and is clean. 64 samples took about 300 s and looks the same once denoised. Time grows with the texels and the samples, so 2048 px costs about four times 1024 px.

## Skipping the bake

An asset can skip the bake. It then keeps its own material lighting, still shadows the baked models, and ships with no lightmap and no second UV set. That is smaller and right for provider scans, whose many small UV islands bake into flat lit facets. Meshes whose every material glows on its own get no lightmap either.
