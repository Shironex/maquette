---
title: MCP tools
description: Every tool the built in MCP server offers, what it does, and whether it spends credits or needs approval in the app.
sidebar:
  label: MCP tools
---

The server offers 65 tools. Set it up first on [Connect an agent](/docs/agents/connect/). Tools that work on a project take its id as `project`, as `list_projects` returns it.

How to read the tables: **Spends** says whether a call costs anything. **Approval** says whether a person has to answer in the Maquette app before it goes on.

## Read only

These look at the workspace and change nothing.

| Tool                                          | What it returns                                                                                                                                                                                                                                                   | Spends  | Approval |
| --------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- | -------- |
| [`list_projects`](#list_projects)             | Every project: template, target (website or game), site or game repo, asset count, what runs in it and which is open in the app                                                                                                                                   | Nothing | No       |
| `get_project`                                 | One project's manifest: name, template, site repo, source folder and its Board assets with every version                                                                                                                                                          | Nothing | No       |
| `list_assets`                                 | A project's Board assets: kind, stage, status, version count and the latest version's file and stats. Takes an optional `stage` filter                                                                                                                            | Nothing | No       |
| [`get_pipeline`](#get_pipeline)               | Steps, validation issues, compression presets, model sizes, the run in progress and the latest runs                                                                                                                                                               | Nothing | No       |
| [`get_run`](#get_run)                         | One run's status and the end of its log                                                                                                                                                                                                                           | Nothing | No       |
| `list_inbox`                                  | The models waiting in the inbox, with kind, size, triangle count and worst check                                                                                                                                                                                  | Nothing | No       |
| `get_inbox_checks`                            | The checks for one inbox model (triangles, textures, rig, scale, materials) and its analysis report                                                                                                                                                               | Nothing | No       |
| `get_concepts`                                | A direction's prompt, refs and images with stars and picks, plus queued, running, waiting and failed jobs                                                                                                                                                         | Nothing | No       |
| `get_provider_job`                            | A send to 3D: status, phase, progress, credits spent and message. With `project` instead of `id`, every send in the project                                                                                                                                       | Nothing | No       |
| `cancel_provider_jobs`                        | Stops sends by id (`ids`, 1 to 50). A by hand send stops waiting, a send not yet started is cancelled, and each id answers on its own. Sends waiting for approval are refused: you deny those in the app                                                          | Nothing | No       |
| [`get_credits`](#get_credits)                 | Each provider's balance, totals for 7 and 30 days and the latest paid sends                                                                                                                                                                                       | Nothing | No       |
| `list_handoffs`                               | The By hand sends waiting for a model, the export file name each waits for, and what the Downloads watcher matched                                                                                                                                                | Nothing | No       |
| `list_removed_assets`                         | A project's removed assets waiting in the workspace trash, newest first                                                                                                                                                                                           | Nothing | No       |
| `get_real_size`                               | An asset's real size and fix-ups, what its model measures, and the defaults new fix-ups start from                                                                                                                                                                | Nothing | No       |
| `list_fixups`                                 | An asset's fix-ups in the order they apply, with the last preview's numbers                                                                                                                                                                                       | Nothing | No       |
| [`get_material_tweaks`](#get_material_tweaks) | An asset's material names, tweaks and per map group codecs, with raw and shipped size                                                                                                                                                                             | Nothing | No       |
| `get_bake_settings`                           | Lightmap size, samples, denoiser, whether `bake_light.py` can denoise, and the assets that skip the bake                                                                                                                                                          | Nothing | No       |
| `get_starters`                                | Each starter script in `tools/` as current, outdated, edited or missing                                                                                                                                                                                           | Nothing | No       |
| `get_layout`                                  | Where each model stands, the camera stops, the desk, the first view, object states, explode tracks, paths and clips kept off the site                                                                                                                             | Nothing | No       |
| `get_motion`                                  | Every model's rig, clips and parts, and what the clips weigh inside the scene budget                                                                                                                                                                              | Nothing | No       |
| `list_built_assets`                           | Each built asset's script, helpers, state and last build                                                                                                                                                                                                          | Nothing | No       |
| `get_page_preview`                            | The site dev server's state, address, command, who started it and its last log lines                                                                                                                                                                              | Nothing | No       |
| `get_game`                                    | A game project's settings and each model against its folder's budget                                                                                                                                                                                              | Nothing | No       |
| `scan_game_models`                            | Every model in a game project's model folder with its numbers, budget check and the asset it was imported as                                                                                                                                                      | Nothing | No       |
| `get_ship_plan`                               | What Ship would write: for a game, new, replaced (marked when an imported original), unchanged and hand made files and the uncommitted file count; for a website, each file written or held back and why, with sizes before and after, and the site's other files | Nothing | No       |
| [`get_ship_settings`](#get_ship_settings)     | A website's Ship style, texture format and quality, each asset's choice and file name, the site folders, the plan and the size report                                                                                                                             | Nothing | No       |
| [`get_size_report`](#get_size_report)         | Each model's source bytes and triangles against what ships, with totals, as rows, CSV and Markdown                                                                                                                                                                | Nothing | No       |
| [`list_library`](#list_library)               | The library's models, with their versions, tags and the projects that use them. Takes an optional `query`, `tag` and `project`                                                                                                                                    | Nothing | No       |
| Tool                                          | What it returns                                                                                                                                                                                                                                                   | Spends  | Approval |
| --------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- | -------- |

`preview_fixups` is read only too: it runs an asset's saved fix-ups headless on a copy and returns the numbers before and after each. It spends nothing.

## Changes the project, spends nothing

| Tool                                                  | What it does                                                                                                      | Spends  | Approval |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- | ------- | -------- |
| [`set_compression_presets`](#set_compression_presets) | Changes the per model texture size, data map size, codec and meshopt level                                        | Nothing | No       |
| [`set_material_tweaks`](#set_material_tweaks)         | Changes an asset's tint, roughness, metalness, emissive strength and map codecs                                   | Nothing | No       |
| [`set_ship_settings`](#set_ship_settings)             | Changes a website's Ship style, texture format, quality, per asset choice and file name, and the site folders     | Nothing | No       |
| `set_bake_settings`                                   | Changes the lightmap size (256 to 4096), samples (1 to 4096) and denoise                                          | Nothing | No       |
| `set_skip_bake`                                       | Sets one or more assets to skip the bake, or join it again                                                        | Nothing | No       |
| `set_pad_edges`                                       | Pads an asset's texture edges before the textures shrink, or stops                                                | Nothing | No       |
| `set_real_size`                                       | Sets or clears the real size a model is scaled to                                                                 | Nothing | No       |
| `add_fixups`                                          | Adds fix-ups to an asset; one of a kind already there is replaced                                                 | Nothing | No       |
| `remove_fixups`                                       | Removes fix-ups by kind, or all of them                                                                           | Nothing | No       |
| `update_starters`                                     | Brings outdated or missing starter scripts up to date, as the Update banner does                                  | Nothing | No       |
| [`set_layout`](#set_layout)                           | Changes placements, camera stops, desk, object states, explode tracks, paths and clips                            | Nothing | No       |
| [`pick_concepts`](#pick_concepts)                     | Puts Lab concepts on the Board at Refs                                                                            | Nothing | No       |
| `star_concepts`                                       | Stars or unstars Lab concepts                                                                                     | Nothing | No       |
| [`accept_inbox_model`](#accept_inbox_model)           | Moves an inbox model into a project as a new asset version                                                        | Nothing | No       |
| `copy_asset`                                          | Copies an asset with its refs and versions to another project                                                     | Nothing | No       |
| `remove_assets`                                       | Takes up to 50 assets off the Board into the workspace trash, stopping their waiting sends                        | Nothing | No       |
| `restore_asset`                                       | Puts a removed asset back from the workspace trash with its files, place and settings                             | Nothing | No       |
| `import_game_models`                                  | Imports models from the game repo, each as an asset whose version 1 is the repo file, and asks for their previews | Nothing | No       |
| `set_game_settings`                                   | Changes a game's format, model folder, export, bake and folder budgets                                            | Nothing | No       |
| `add_built_asset`                                     | Adds a model a Blender script in the project makes, and builds it headless                                        | Nothing | No       |
| `rebuild_built_asset`                                 | Builds one built asset again, or every new or stale one                                                           | Nothing | No       |
| `render_thumbnail`                                    | Renders a studio thumbnail of a model with headless Blender and returns the PNG's path                            | Nothing | No       |
| `start_page_preview`                                  | Starts the site dev server the Page view shows, or reuses the running one                                         | Nothing | No       |
| `stop_page_preview`                                   | Stops that server, whoever started it                                                                             | Nothing | No       |
| `cancel_run`                                          | Cancels a queued or running run. A run waiting for approval is cancelled too                                      | Nothing | No       |
| [`add_to_library`](#add_to_library)                   | Copies an asset's newest model into the library                                                                   | Nothing | No       |
| [`use_from_library`](#use_from_library)               | Copies a library model into a project as a new asset                                                              | Nothing | No       |
| [`update_from_library`](#update_from_library)         | Adds a newer library version to an asset as its next version, or returns the plan                                 | Nothing | No       |

## Spends credits or Codex usage, or needs approval

| Tool                            | What it does                                                        | Spends                                              | Approval                                                                  |
| ------------------------------- | ------------------------------------------------------------------- | --------------------------------------------------- | ------------------------------------------------------------------------- |
| [`run_pipeline`](#run_pipeline) | Runs a project's pipeline, optionally from a step or build only     | Nothing                                             | At the Ship step, when Ask before shipping is on or the project is a game |
| [`send_to_3d`](#send_to_3d)     | Sends an asset's refs to Tripo or Meshy, or writes a By hand folder | Credits for Tripo and Meshy. By hand spends nothing | Always, for Tripo and Meshy                                               |
| `send_batch_to_3d`              | Sends up to 24 assets at once with the same provider                | Credits for Tripo and Meshy                         | One approval for the whole batch                                          |
| `redo_inbox_model`              | Sends an inbox model's asset again, prefilled from its send         | Credits for Tripo and Meshy                         | Always, for Tripo and Meshy                                               |
| `redo_inbox_models`             | Redoes several inbox models from one project as one batch           | Credits for Tripo and Meshy                         | One approval for the whole batch                                          |
| `start_concepts`                | Starts concept image jobs in the Lab                                | Codex usage                                         | No                                                                        |
| `make_object_refs`              | Makes one isolated image per object of a concept                    | Codex usage                                         | No                                                                        |
| `retry_concept_jobs`            | Runs failed or cancelled Lab jobs again                             | Codex usage                                         | No                                                                        |

## The main tools

### list_projects

`read only`

Lists the projects in the open workspace: template, site repo, created date, asset count, the picture the app shows, what runs in it now and which one is open in the app. Takes no parameters. Every other tool takes a project id from here.

### get_pipeline

`read only`

Reads a project's steps and their dependencies, validation issues, the compression presets (with lock notes), model sizes, the run in progress and the latest runs.

| Parameter | Type   | Required | Notes           |
| --------- | ------ | -------- | --------------- |
| `project` | string | yes      | The project id. |

```json
{ "project": "moon-island" }
```

### run_pipeline

`needs approval at Ship`

Runs a project's pipeline, optionally from one step and optionally ignoring the cache. A full run (no `from`) first rebuilds the built assets whose script or helpers changed, and is refused when one of them fails. It waits up to `waitSeconds`, then answers with the run's status. The run carries on either way.

With Ask before shipping on, and always in a [game project](/docs/start/game-projects/), the run stops before Ship and answers `waiting for approval` until you approve or deny it in the app. Poll [`get_run`](#get_run) instead of starting the run again.

| Parameter     | Type    | Required | Notes                                                                       |
| ------------- | ------- | -------- | --------------------------------------------------------------------------- |
| `project`     | string  | yes      | The project id.                                                             |
| `from`        | string  | no       | A step id. It runs even when cached, and so does everything after it.       |
| `force`       | boolean | no       | Ignore the cache and run every step.                                        |
| `buildOnly`   | boolean | no       | Stop before Ship: the run builds everything and writes nothing to the site. |
| `waitSeconds` | number  | no       | How long to wait before answering. 20 by default, 120 at most.              |

```json
{ "project": "moon-island", "from": "compress" }
```

A build only run never waits for approval, since it has no Ship step. Its summary carries `"buildOnly": true`.

### get_run

`read only`

Reads one run's status and the end of its log. The status is `queued`, `running`, `done`, `failed`, `cancelled` or `waiting for approval`.

| Parameter  | Type   | Required | Notes                                                                            |
| ---------- | ------ | -------- | -------------------------------------------------------------------------------- |
| `project`  | string | yes      | The project id.                                                                  |
| `number`   | number | no       | The run number ("Run 12" is 12). Without it and without `runId`, the newest run. |
| `runId`    | string | no       | The run id `run_pipeline` returned, such as `frostveil-run-12`.                  |
| `logLines` | number | no       | Log lines from the end. 40 by default, 400 at most.                              |

### send_to_3d

`costs credits` `needs approval`

Sends an asset's refs to 3D. `tripo` and `meshy` spend credits, so the send always waits for you to approve it in the Maquette app and answers `waiting for approval`. Follow it with `get_provider_job`. `by-hand` spends nothing: it writes a handoff folder with the images and a settings card, and the asset waits for a person to bring the GLB back.

| Parameter   | Type     | Required | Notes                                                                                                             |
| ----------- | -------- | -------- | ----------------------------------------------------------------------------------------------------------------- |
| `project`   | string   | yes      | The project id.                                                                                                   |
| `asset`     | string   | yes      | The asset id, as `list_assets` returns it.                                                                        |
| `provider`  | string   | yes      | `tripo`, `meshy` or `by-hand`.                                                                                    |
| `service`   | string   | no       | By hand only: whose web app the settings card is for, `tripo` by default.                                         |
| `variant`   | string   | no       | `prop` or `character`. By default from the asset's kind.                                                          |
| `images`    | string[] | no       | Images relative to the project folder, front first, at most 4. By default the asset's own refs.                   |
| `overrides` | object   | no       | Preset fields for this send only, such as `{"faceLimit": 20000}`. An unknown name is refused with the valid list. |
| `prompt`    | string   | no       | Meshy only: text that guides the texture, at most 800 characters.                                                 |

```json
{ "project": "moon-island", "asset": "far-spire", "provider": "tripo" }
```

### set_compression_presets

`changes the project`

Sets compression presets, as the Pipeline view's Compression presets panel does. Only the fields you give change. Locked rows are refused with their lock note, and an unknown model with the list of valid ones. It spends nothing. See [Compress](/docs/pipeline/compress/).

| Parameter | Type     | Required | Notes               |
| --------- | -------- | -------- | ------------------- |
| `project` | string   | yes      | The project id.     |
| `presets` | object[] | yes      | The rows to change. |

Each row takes `model` (the GLB file stem from `get_pipeline`, or the asset's name when it has one model) and any of `textureSize` (512, 1024, 2048 or 4096), `dataTextureSize` (the same sizes, or `null` to use `textureSize`), `codec` (`etc1s` or `uastc`) and `meshopt` (`low`, `medium` or `high`).

```json
{
  "project": "moon-island",
  "presets": [{ "model": "vending", "textureSize": 1024, "meshopt": "medium" }]
}
```

### get_material_tweaks

`read only`

Reads an asset's material tweaks and texture codecs: the names of its model's materials, the tweaks stored for every material (`all`) and per material name, and the codec each map group (`color`, `normal`, `other`) is compressed with, with the model's raw and shipped size.

| Parameter | Type   | Required | Notes           |
| --------- | ------ | -------- | --------------- |
| `project` | string | yes      | The project id. |
| `asset`   | string | yes      | The asset id.   |

### set_material_tweaks

`changes the project`

Changes an asset's material tweaks and codecs, as the Inspector's Material panel does. `all` changes every material and `materials` changes one by name, and its fields win over `all`. Everything is checked before anything is written. A field left out keeps its value and `null` clears it.

| Parameter   | Type    | Required | Notes                                                                              |
| ----------- | ------- | -------- | ---------------------------------------------------------------------------------- |
| `project`   | string  | yes      | The project id.                                                                    |
| `asset`     | string  | yes      | The asset id.                                                                      |
| `reset`     | boolean | no       | Drop the asset's tweaks first.                                                     |
| `all`       | object  | no       | Tweaks for every material.                                                         |
| `materials` | object  | no       | Tweaks per material name. A name the model does not have is refused with the list. |
| `codecs`    | object  | no       | `color`, `normal` and `other`, each `etc1s`, `uastc` or `null`.                    |

A tweak takes `tint` (`"#rrggbb"`, multiplied into the base color), `roughness` (`{ "mode": "override" | "multiply", "value": 0.4 }`), `metalness` (0 to 1) and `emissiveStrength` (0 to 50). Overriding roughness or metalness drops the model's metal and roughness map.

### get_ship_settings

`read only`

Reads how a website project ships: the style, texture format and quality, each asset's choice and file name, and the site folders. It also returns the dry-run plan of the next Ship and the size report, as the [Ship settings](/docs/pipeline/ship/#ship-settings) view shows them. `scriptReady` and `originReady` say whether the project's starters support the choices and the own origin export; when one is false, run `update_starters`. A game project is refused: use `get_ship_plan`.

| Parameter | Type   | Required | Notes           |
| --------- | ------ | -------- | --------------- |
| `project` | string | yes      | The project id. |

### set_ship_settings

`changes the project`

Changes how a website project ships. Only the fields you give change, and everything is checked before anything is written. It returns what `get_ship_settings` returns.

| Parameter  | Type           | Required | Notes                                                                                                                                        |
| ---------- | -------------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `project`  | string         | yes      | The project id.                                                                                                                              |
| `style`    | string         | no       | `scene`, or `models` for a hand-built site.                                                                                                  |
| `textures` | string         | no       | `ktx2`, `webp` or `avif`.                                                                                                                    |
| `quality`  | number         | no       | The WebP and AVIF quality, 1 to 100.                                                                                                         |
| `assets`   | array          | no       | `{ "asset", "choice", "file" }` per asset. `choice` is `auto`, `never` or `always`; `file` is a name without `.glb`, `null` for the default. |
| `models`   | string         | no       | The site's models folder, relative to the site. Written to `maquette.site.json` with `posters` and `data`.                                   |
| `posters`  | string or null | no       | The posters folder, `null` for none.                                                                                                         |
| `data`     | string or null | no       | The scene data file, `null` for none.                                                                                                        |

```json
{
  "project": "room",
  "style": "models",
  "textures": "webp",
  "models": "apps/landing/public/room/models",
  "posters": null,
  "data": null,
  "assets": [{ "asset": "desk", "choice": "never" }]
}
```

### get_size_report

`read only`

Returns each model's source file (bytes and triangles) against the file that ships, the totals over the models measured on both sides, and the same table as `csv` and `markdown`. Shipped numbers are empty before the first run.

| Parameter | Type   | Required | Notes           |
| --------- | ------ | -------- | --------------- |
| `project` | string | yes      | The project id. |

### set_layout

`changes the project`

Changes a project's scene layout, as the Layout editor does. Only what you name changes, and the whole result is checked before anything is written. Unknown assets or stops are refused. The open app updates live, and the next Build scene and Capture posters use it. Returns the layout like `get_layout`.

| Parameter   | Type           | Required | Notes                                                                                                             |
| ----------- | -------------- | -------- | ----------------------------------------------------------------------------------------------------------------- |
| `project`   | string         | yes      | The project id.                                                                                                   |
| `models`    | object[]       | no       | `{ asset, position?, rotation?, scale?, remove? }`. Position in metres, y up. Rotation in degrees.                |
| `stops`     | object[]       | no       | `{ id?, name?, position?, target?, fov?, models?, remove? }`. Without an `id`, it adds a stop.                    |
| `stopOrder` | string[]       | no       | Every stop id in the new tour order.                                                                              |
| `desk`      | object or null | no       | The desk size, or `null` to remove it.                                                                            |
| `states`    | object[]       | no       | `{ asset, states, transitions?, triggers?, remove? }`. Sets an asset's object states whole.                       |
| `clips`     | object[]       | no       | `{ asset, clip, ship }`. `ship: false` keeps a clip off the site.                                                 |
| `explode`   | object[]       | no       | `{ asset, distance?, easing?, keys, remove? }`. Sets an asset's explode track whole.                              |
| `paths`     | object[]       | no       | `{ id?, name?, curve?, points?, start?, end?, width?, color?, glow?, remove? }`. Adds, changes or removes a path. |
| `energize`  | object[]       | no       | `{ path, easing?, keys, remove? }`. Sets a path's energize track whole.                                           |

```json
{ "project": "moon-island", "models": [{ "asset": "bench", "rotation": [0, 90, 0] }] }
```

The Timeline view's tracks go through `set_layout` too. Its state changes are an asset's stop and scroll `triggers` in `states`. An explode track's `keys` are `{ stop, amount }`, at most one per camera stop, with `amount` 0 (together) to 1 (spread `distance` metres, 0.25 when left out) and `easing` between them (`ease-in-out` when left out).

```json
{
  "project": "moon-island",
  "explode": [
    {
      "asset": "switch-cabinet",
      "distance": 0.3,
      "keys": [
        { "stop": "canopy", "amount": 0 },
        { "stop": "pullback", "amount": 1 }
      ]
    }
  ]
}
```

A path is a list of 2 to 128 `points` in metres, y up. Without an `id`, or with one that is new, it adds a path, which needs a `name` and `points`; `curve` is `catmull-rom` (smooth, the default) or `polyline`, `width` is in metres (0.035 when left out) and `color` and `glow` are `#rrggbb`. `start` and `end` name the asset that end follows from then on, and `null` lets it go: when that model's placement moves, the end moves by as much. An energize track's `keys` are `{ stop, amount }`, at most one per camera stop, with `amount` 0 (dark) to 1 (lit from the first point to the last) and `easing` between them (`linear` when left out). Removing a path drops its track.

```json
{
  "project": "moon-island",
  "paths": [
    {
      "name": "Lantern cable",
      "points": [
        [-3.2, 0, 3.4],
        [0, 0.05, 5.2],
        [3.6, 0, 1.2]
      ],
      "start": "shop-house",
      "end": "vending",
      "width": 0.06
    }
  ],
  "energize": [
    {
      "path": "lantern-cable",
      "keys": [
        { "stop": "sakura", "amount": 0 },
        { "stop": "canopy", "amount": 1 }
      ]
    }
  ]
}
```

### pick_concepts

`changes the project`

Picks Lab concepts onto the Board, as Send picks to Refs does. Each image is copied into `refs/` and becomes, or joins, an asset at Refs. An image picked before keeps its ref and answers with the same asset, so calling again makes no duplicates. Every image is checked before anything is written. It spends nothing, and the Lab and Board update live.

| Parameter | Type     | Required | Notes           |
| --------- | -------- | -------- | --------------- |
| `project` | string   | yes      | The project id. |
| `images`  | object[] | yes      | 1 to 24 images. |

Each image is `{ file | direction + index, star?, name?, kind? }`. `file` is the path `get_concepts` lists. `name` is the asset's name, at most 80 characters, and defaults to the object's name for object refs, else the direction's. `kind` is `prop`, `character` or `scene` for a new asset only.

### accept_inbox_model

`changes the project`

Moves a model from the inbox into a project as a new asset version on the Board, and renders its thumbnail headless. Any By hand send of that asset still waiting for its model closes as done, so `list_handoffs` no longer lists it; a Tripo or Meshy send runs on.

| Parameter | Type   | Required | Notes                                                |
| --------- | ------ | -------- | ---------------------------------------------------- |
| `id`      | string | yes      | The inbox item id, its file name, from `list_inbox`. |
| `project` | string | yes      | The project that gets the model.                     |
| `asset`   | string | no       | An existing asset id to add a version to.            |
| `name`    | string | no       | The asset name, when a new asset is made.            |

### get_credits

`read only`

Reads each provider's balance (a free call, skipped without a saved key), totals for the last 7 and 30 days with the share spent on redos, and the most recent paid sends from the local ledger.

| Parameter | Type    | Required | Notes                                                       |
| --------- | ------- | -------- | ----------------------------------------------------------- |
| `project` | string  | no       | Only this project's spends.                                 |
| `limit`   | number  | no       | How many recent spends to list. 20 by default, 200 at most. |
| `balance` | boolean | no       | Ask Tripo and Meshy for the balance. `true` by default.     |

### list_library

`read only`

Lists the models in the [Library](/docs/views/library/): name, kind, tags, versions and the projects that use each. With `project`, it also lists that project's assets that have an update waiting.

| Parameter | Type   | Required | Notes                                                   |
| --------- | ------ | -------- | ------------------------------------------------------- |
| `query`   | string | no       | Matches a model's name, tag or project.                 |
| `tag`     | string | no       | Only models with this tag.                              |
| `project` | string | no       | Also list this project's assets with an update waiting. |

### add_to_library

`changes the library`

Copies an asset's newest model, picture, refs, real size, fix-ups and material tweaks into the library, as Add to library does. The same model again (the same hash) adds nothing, and a newer model of an asset already there adds the next version. It spends nothing and the app updates live.

| Parameter | Type     | Required | Notes                                       |
| --------- | -------- | -------- | ------------------------------------------- |
| `project` | string   | yes      | The project id.                             |
| `asset`   | string   | yes      | The asset id or name.                       |
| `name`    | string   | no       | The entry's name.                           |
| `tags`    | string[] | no       | Lowercased, at most 16, 32 characters each. |

```json
{ "project": "moon-island", "asset": "bench", "tags": ["furniture"] }
```

### use_from_library

`changes the project`

Copies a library model into a project as a new asset at v1, with its picture, refs, real size, fix-ups and material tweaks. A name already taken gets a number. A website project refuses an `.obj` entry with the reason. It spends nothing.

| Parameter | Type   | Required | Notes                                       |
| --------- | ------ | -------- | ------------------------------------------- |
| `entry`   | string | yes      | The entry id, as `list_library` returns it. |
| `project` | string | yes      | The project id.                             |
| `version` | number | no       | The version to copy. The newest by default. |

### update_from_library

`changes the project`

Adds the library's newer version to an asset copied from it, as the asset's next version. Older versions stay. Real size, fix-ups and material tweaks carry over except what no longer applies. With `dryRun`, it returns that plan and copies nothing. It spends nothing.

| Parameter | Type    | Required | Notes                             |
| --------- | ------- | -------- | --------------------------------- |
| `project` | string  | yes      | The project id.                   |
| `asset`   | string  | yes      | The asset id or name.             |
| `dryRun`  | boolean | no       | Return the plan and copy nothing. |
