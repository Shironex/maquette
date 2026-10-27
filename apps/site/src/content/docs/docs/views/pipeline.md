---
title: Pipeline view
description: Run a project's steps, see what is cached and tune compression and baked light.
sidebar:
  label: Pipeline
---

The Pipeline view runs the steps in the project's `pipeline.json`. A new Floating diorama or Room project starts with [Build scene](/docs/pipeline/build-scene/), [Bake light](/docs/pipeline/bake-light/), [Export GLB](/docs/pipeline/export-glb/), [Compress](/docs/pipeline/compress/), [Capture posters](/docs/pipeline/capture-posters/) and [Ship](/docs/pipeline/ship/).

## Steps

A step has an `id`, a `kind` (`blender`, `shell`, `gltf-transform`, `toktx` or `ffmpeg`), a script or command, an `args` array, `inputs` and `outputs` globs and `dependsOn`. Every field is in the [pipeline.json reference](/docs/pipeline/pipeline-json/).

- Programs are spawned directly with an args array, never through a shell. On Windows they start inside a job object, so Cancel ends the whole process tree.
- Blender always runs in the background. A `blender` step without `-b` is refused. The only time Maquette opens Blender's window is when you click Open in Blender.
- A step is skipped when its input files, script, args and tool version hash to the same key as the last good run. Run from here starts at a step, and Ignore the cache reruns everything.
- One run at a time per project; more runs queue. The last 5 runs and their logs are kept in the project's cache folder.
- A running step shows a progress bar when its tool reports how far it is.

## Panels

Compression presets sets the texture size, data map size, codec and meshopt level per model, as [Compress](/docs/pipeline/compress/) reads them. Each row also shows what the last run made of that model, its raw export and the size it ships at, and the line above the table adds them up for every model. Sort the table by name, shipped size or raw size. **Size report** opens Ship settings at the size report, with the budgets and the dry run plan. Drag the edge between the log and the presets (or focus it and use the arrow keys) to give either more room; double click puts it back.

Baked light sets the lightmap size, samples and denoiser, as [Bake light](/docs/pipeline/bake-light/) reads them. When every model skips the bake, or the project ships to a hand-built site, the card folds to one line that says why. **Bake settings** opens it again.

**Preview on site** starts the site's dev server and opens it in the browser. While the server runs the button splits into **Open preview** and **Stop**, and the status bar item opens a menu to open it, show it in the Page view or stop it. See [Website projects](/docs/start/website-projects/).

## Starter scripts

A new project gets small, commented scripts in `tools/`, so it runs end to end on day one. They are the project's to edit, and Maquette never writes over a file that exists on its own. When newer starters ship, the view says "New starter scripts available" and Update brings the untouched ones up to date. An edited script is never overwritten silently: choose Replace with the new starter (your file is kept as `<name>.py.bak`) or Keep mine.

## Game projects

A [game project](/docs/start/game-projects/) runs Build models, Bake light (off by default), Export, Game export and Ship. The Game export panel takes the place of the compression presets, and when Ship waits the view opens the Ship diff with Approve and ship and Deny.
