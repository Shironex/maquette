---
title: Export GLB
description: Writes one uncompressed GLB per model into web/raw, with its lightmap as the emissive texture.
sidebar:
  label: Export GLB
---

Step 3 of 6 in a new project. Kind: `blender`. Script: `tools/export_web.py`.

## What it does

It writes one GLB per model collection, and per other top level object, into `web/raw/`, named in snake case. The files are uncompressed, for [Compress](/docs/pipeline/compress/). The step clears `web/raw` before it writes.

A baked object keeps both UV sets (`lightmap` is `TEXCOORD_1`), carries its lightmap as the emissive texture on that UV, and has the material extras `{"lightmap": <scale>, "lightmapUv": 1}`. A material that glows on its own keeps its emission as it is, and a mesh that was not baked ships without the `lightmap` UV.

Rigged models keep their armature, skin and clips: every action becomes one glTF animation under its own name.

## Inputs and outputs

**Inputs:** `blender/scene_baked.blend` and `blender/bake/*`.

**Output:** `web/raw/*.glb`.

The Inspector's Export GLB re-exports one selected model on its own, and copies that file over the raw export.
