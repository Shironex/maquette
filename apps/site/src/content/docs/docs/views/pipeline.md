---
title: Pipeline view
description: Run a project's steps, see what is cached and tune compression and baked light.
sidebar:
  label: Pipeline
---

The Pipeline view runs the steps in the project's `pipeline.json`. A new Floating diorama or Room project starts with [Build scene](/docs/pipeline/build-scene/), [Bake light](/docs/pipeline/bake-light/), [Export GLB](/docs/pipeline/export-glb/), [Compress](/docs/pipeline/compress/), [Capture posters](/docs/pipeline/capture-posters/) and [Ship](/docs/pipeline/ship/).

## Steps

A step has an `id`, a `kind` (`blender`, `shell`, `gltf-transform`, `toktx` or `ffmpeg`), a script or command, an `args` array, `inputs` and `outputs` globs and `dependsOn`.

- Programs are spawned directly with an args array, never through a shell. On Windows they start inside a job object, so Cancel ends the whole process tree.
- Blender always runs in the background. A `blender` step without `-b` is refused. The only time Maquette opens Blender's window is when you click Open in Blender.
- A step is skipped when its input files, script, args and tool version hash to the same key as the last good run. Run from here starts at a step, and Ignore the cache reruns everything.
- One run at a time per project; more runs queue. The last 5 runs and their logs are kept in the project's cache folder.
- A running step shows a progress bar when its tool reports how far it is.

## Panels

Compression presets sets the texture size, codec and meshopt level per model, as [Compress](/docs/pipeline/compress/) reads them. Baked light sets the lightmap size, samples and denoiser, as [Bake light](/docs/pipeline/bake-light/) reads them.

**Preview on site** starts the site's dev server and opens it in the browser. See [Website projects](/docs/start/website-projects/).

## Starter scripts

A new project gets small, commented scripts in `tools/`, so it runs end to end on day one. They are the project's to edit, and Maquette never writes over a file that exists on its own. When newer starters ship, the view says "New starter scripts available" and Update brings the untouched ones up to date. An edited script is never overwritten silently: choose Replace with the new starter (your file is kept as `<name>.py.bak`) or Keep mine.
