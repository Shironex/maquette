---
title: Ship
description: Copies the compressed models and posters into your site repo and writes the scene data.
sidebar:
  label: Ship
---

Step 6 of 6 in a new project. Kind: `shell`. Command: `node`. Script: `tools/ship.mjs`.

## What it does

It copies `web/dist/*.glb` and the posters into the project's site repo and writes the site's scene data when the site asks for it. Without a site repo it says so and passes. Where things go is set in [`maquette.site.json`](/docs/start/website-projects/#maquettesitejson).

Ship has no outputs inside the project, so it runs every time. To build everything and stop before Ship, tick **Build only** next to Run pipeline, or pass `buildOnly` to [`run_pipeline`](/docs/agents/mcp-tools/#run_pipeline). The run then lists Ship as left out.

## Ship settings

**Ship settings** in the Pipeline view's top bar opens the settings, the plan and the size report. They are saved in the project's `ship.json`, and the folders go into the site's `maquette.site.json`. You can set them before the first export: every asset with a model is listed already. An agent reads and changes the same settings with [`get_ship_settings`](/docs/agents/mcp-tools/#get_ship_settings) and [`set_ship_settings`](/docs/agents/mcp-tools/#set_ship_settings).

| Setting        | Values                           | What it does                                                                                                                                      |
| -------------- | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Style          | Scene, Hand-built site           | Scene keeps every model where the layout put it, under versioned names, with scene data. Hand-built site is for a page that places models itself. |
| Texture format | KTX2, WebP, AVIF                 | What [Compress](/docs/pipeline/compress/) turns the textures into. WebP and AVIF need no transcoder on the page. The sizes per model still apply. |
| Image quality  | 1 to 100, 80 by default          | The WebP or AVIF quality. KTX2 has its own codec column in the presets instead.                                                                   |
| Ship, per file | Auto, Never, Always              | When Ship writes that model. See below.                                                                                                           |
| File name      | `a` to `z`, `0` to `9`, `_`, `-` | The name the model ships as, without `.glb`. Empty keeps the default name.                                                                        |
| Folders        | Models, posters, data            | Written into `maquette.site.json`, keeping its other keys. Empty posters copies none; empty data writes none.                                     |

A project without `ship.json` ships as it always did: Scene style, KTX2 textures, versioned names.

### A hand-built site

The Hand-built site style suits a page that loads one GLB per model and places it in its own code. It changes three things:

- **Own origin.** Export GLB undoes the layout's move for each model, so every file is centred where its model was made, not where the scene put it.
- **Stable names.** Each model ships under its asset id (`shop-house` ships as `shop_house.glb`), or the file name you chose, with no version in it. A new version replaces the same file.
- **Smaller files stay.** A site file that is smaller than the new one is held back, so a model you shrank by hand is not replaced by a bigger one.

Set the data file to empty, since such a site has no scene data. The own origin needs the current `build_scene.py` and `export_web.py`, and the choices need the current `ship.mjs`: the Update banner in the Pipeline view brings them up to date, and the plan's checks say when one is missing.

### Auto, Never and Always

| Choice | Ship writes the file                                                                                                               |
| ------ | ---------------------------------------------------------------------------------------------------------------------------------- |
| Auto   | Unless the site's file was not made by Maquette (it is not in the ship record), or it is smaller and the style is Hand-built site. |
| Never  | Never. The site keeps its own file.                                                                                                |
| Always | Always, whoever made the site's file.                                                                                              |

A held file is not copied and not removed, and the log says how many were held back.

### The plan

The plan is a dry run of the next Ship: for each file, whether it is new, replaces a site file or is held back and why, with the bytes and triangles before and after. Below it are the site's other files, left alone or removed because Maquette shipped them before. [`get_ship_plan`](/docs/agents/mcp-tools/) returns the same plan.

### The size report

One row per model: the source file's bytes and triangles against what ships. The totals count only models measured on both sides. **Copy CSV** and **Copy Markdown** copy it, for a pull request or a note, and [`get_size_report`](/docs/agents/mcp-tools/#get_size_report) returns it with both texts.

## Safe by design

- It checks every folder before it copies anything, and stops before an empty layout could wipe the site's scene.
- With a data file, it copies only the models the data lists, so a leftover in `web/dist` never reaches the site.
- It keeps `web/ship-record.json` with the files it shipped into each site folder. A file it shipped last time and no longer ships is removed from the site and logged. Every other file in the site is left alone.
- A site file that is not in the record, so Maquette did not make it, is never replaced unless you set that model to Always.
- It never commits.

:::caution[Ask before shipping]
With Ask before shipping on (the default), a run an agent starts over MCP stops before Ship and the app shows Approve or Deny. Until you answer, `run_pipeline` and `get_run` report `waiting for approval`.
:::
