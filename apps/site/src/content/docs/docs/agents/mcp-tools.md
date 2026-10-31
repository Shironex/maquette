---
title: MCP tools
description: Every tool the built in MCP server offers, what it does, and whether it spends credits or needs approval in the app.
sidebar:
  label: MCP tools
---

The server offers 101 tools. Set it up first on [Connect an agent](/docs/agents/connect/). Tools that work on a project take its id as `project`, as `list_projects` returns it.

How to read the tables: **Spends** says whether a call costs anything. **Approval** says whether a person has to answer in the Maquette app before it goes on.

## Read only

These look at the workspace and change nothing.

| Tool                                          | What it returns                                                                                                                                                                                                                                                                                                                                                        | Spends  | Approval |
| --------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- | -------- |
| [`list_projects`](#list_projects)             | Every project: template, target (website or game), site or game repo, asset count, what runs in it and which is open in the app                                                                                                                                                                                                                                        | Nothing | No       |
| `get_project`                                 | One project's manifest: name, template, site repo, source folder and its Board assets with every version                                                                                                                                                                                                                                                               | Nothing | No       |
| `list_assets`                                 | A project's Board assets: kind, stage, status, version count, the latest version's file and stats, and its refs with their views (`refsTagged` false while they are read from the refs' order). Takes an optional `stage` filter                                                                                                                                       | Nothing | No       |
| [`get_pipeline`](#get_pipeline)               | Steps, validation issues, compression presets, model sizes, the run in progress and the latest runs                                                                                                                                                                                                                                                                    | Nothing | No       |
| [`get_run`](#get_run)                         | One run's status and the end of its log                                                                                                                                                                                                                                                                                                                                | Nothing | No       |
| `list_inbox`                                  | The models waiting in the inbox, with kind, size, triangle count, worst check, the version each would become and the newer version that replaced it, plus the discarded models                                                                                                                                                                                         | Nothing | No       |
| `get_inbox_checks`                            | The checks for one inbox model (triangles, textures, rig, VRM readiness, scale, materials) and its analysis report (with `vrm`: the bone map, rest pose measures and findings), read against its send as `list_inbox` does                                                                                                                                             | Nothing | No       |
| `get_concepts`                                | A direction's prompt, refs and images with stars and picks, plus queued, running, waiting and failed jobs                                                                                                                                                                                                                                                              | Nothing | No       |
| `get_style_board`                             | A project's style board: the style line, the palette, up to 4 images with the concept or ref each came from, and its revision                                                                                                                                                                                                                                          | Nothing | No       |
| `get_codex_usage`                             | How much of Codex's five hour and weekly windows is used, when each resets, whether the usage limit holds and until when. `known` is false with a reason when this Codex CLI cannot say. Lab jobs waiting for the limit move to the reset time it names                                                                                                                | Nothing | No       |
| [`get_image_backend`](#set_image_backend)     | The Lab's backend per kind of job (`codex` or `openai-images`), the Image API's quality and monthly cap in dollars, this month's spend and what queued jobs will likely cost, whether an OpenAI key is saved, and the estimate per image                                                                                                                               | Nothing | No       |
| [`get_ref_views`](#set_ref_views)             | An asset's refs with the view each shows, whether they were tagged, the candidates `make_other_views` and `clean_ref_background` made and their jobs still running                                                                                                                                                                                                     | Nothing | No       |
| `get_provider_job`                            | A send to 3D: status, phase, progress, credits spent and message. With `project` instead of `id`, every send in the project                                                                                                                                                                                                                                            | Nothing | No       |
| `cancel_provider_jobs`                        | Stops sends by id (`ids`, 1 to 50). A by hand send stops waiting, a send not yet started is cancelled, and each id answers on its own. Sends waiting for approval are refused: you deny those in the app                                                                                                                                                               | Nothing | No       |
| `cancel_concept_jobs`                         | Cancels Lab jobs as the Lab queue's Cancel does (`{ project, jobs? }`, every unfinished job without `jobs`): queued, running, waiting out a limit or waiting for approval. Each job answers with its state or why it was not cancelled                                                                                                                                 | Nothing | No       |
| `prioritize_concept_jobs`                     | Run next: moves a project's queued Lab jobs (or the named ones) to the front of the line, ahead of other projects' jobs. Running, waiting and finished jobs are listed as skipped                                                                                                                                                                                      | Nothing | No       |
| [`get_credits`](#get_credits)                 | Each provider's balance, totals for 7 and 30 days and the latest paid sends, and the month's OpenAI image dollars against the cap                                                                                                                                                                                                                                      | Nothing | No       |
| [`list_handoffs`](#list_handoffs)             | The By hand sends waiting for a model, the export file name each waits for, and the Downloads watcher: downloads that need a look (`needsLook`) and ones it could not place (`unmatched`)                                                                                                                                                                              | Nothing | No       |
| `list_removed_assets`                         | A project's removed assets waiting in the workspace trash, newest first                                                                                                                                                                                                                                                                                                | Nothing | No       |
| `get_real_size`                               | An asset's real size and fix-ups, what its model measures, and the defaults new fix-ups start from                                                                                                                                                                                                                                                                     | Nothing | No       |
| `list_fixups`                                 | An asset's fix-ups in the order they apply, with the last preview's numbers                                                                                                                                                                                                                                                                                            | Nothing | No       |
| [`get_material_tweaks`](#get_material_tweaks) | An asset's material names, tweaks and per map group codecs, with raw and shipped size                                                                                                                                                                                                                                                                                  | Nothing | No       |
| [`get_avatar`](#get_avatar)                   | A character asset's VRM avatar: its readiness (bone map, rest pose measures, findings), the meta, bone overrides and material mode, the last normalised rest pose with its sheets and state, the parts that could swing and the last spring chains, and the last export                                                                                                | Nothing | No       |
| `get_bake_settings`                           | Lightmap size, samples, denoiser, whether `bake_light.py` can denoise, and the assets that skip the bake                                                                                                                                                                                                                                                               | Nothing | No       |
| `get_starters`                                | Each starter script in `tools/` as current, outdated, edited or missing                                                                                                                                                                                                                                                                                                | Nothing | No       |
| `get_layout`                                  | Where each model stands, the camera stops, the desk, the first view, object states, explode tracks, paths and clips kept off the site                                                                                                                                                                                                                                  | Nothing | No       |
| `get_motion`                                  | Every model's rig, clips and parts, and what the clips weigh inside the scene budget                                                                                                                                                                                                                                                                                   | Nothing | No       |
| [`get_parts`](#set_parts)                     | An asset's parts as its newest model ships them: the name in the model, the name it ships as, triangles, materials, hidden and joined parts. Runs its split headless on a copy first when it has not run                                                                                                                                                               | Nothing | No       |
| `compare_versions`                            | Two versions of an asset (or one before and after its fix-ups) part by part: the parts Motion can move, draw calls, triangles, textures, transfer size and Inbox checks                                                                                                                                                                                                | Nothing | No       |
| `list_built_assets`                           | Each built asset's script, helpers, state and last build                                                                                                                                                                                                                                                                                                               | Nothing | No       |
| `get_page_preview`                            | The site dev server's state, address, command, who started it and its last log lines                                                                                                                                                                                                                                                                                   | Nothing | No       |
| `get_game`                                    | A game project's settings and each model against its folder's budget                                                                                                                                                                                                                                                                                                   | Nothing | No       |
| `scan_game_models`                            | Every model in a game project's model folder with its numbers, budget check and the asset it was imported as                                                                                                                                                                                                                                                           | Nothing | No       |
| `get_ship_plan`                               | What Ship would write: for a game, new, replaced (marked when an imported original), unchanged and hand made files and the uncommitted file count, and the old files a moved asset leaves (removed); `repo` plans against another checkout or worktree; for a website, each file written or held back and why, with sizes before and after, and the site's other files | Nothing | No       |
| [`get_ship_settings`](#get_ship_settings)     | A website's Ship style, texture format and quality, each asset's choice and file name, the site folders, the plan and the size report                                                                                                                                                                                                                                  | Nothing | No       |
| [`get_size_report`](#get_size_report)         | Each model's source bytes and triangles against what ships, with totals, as rows, CSV and Markdown                                                                                                                                                                                                                                                                     | Nothing | No       |
| [`list_library`](#list_library)               | The library's models, with their versions, tags and the projects that use them. Takes an optional `query`, `tag`, `project`, `collection`, `limit`, `cursor` and `full`                                                                                                                                                                                                | Nothing | No       |
| [`find_look_alikes`](#find_look_alikes)       | Library entries that may be one model twice (same look or same name), the entries with no readable picture, the recent merges and the settings                                                                                                                                                                                                                         | Nothing | No       |
| [`get_pack`](#get_pack)                       | One export pack: its state, each model's check with triangles before and after, and what needs you                                                                                                                                                                                                                                                                     |
| [`list_packs`](#list_packs)                   | The newest export packs and the pack presets                                                                                                                                                                                                                                                                                                                           |
| [`list_animations`](#list_animations)         | The animation library's clips: pack, tags, source rig, bone count, frames, rate, seconds, root motion, loop and size. Takes an optional `query`, `pack`, `tag` and `rig`                                                                                                                                                                                               | Nothing | No       |
| Tool                                          | What it returns                                                                                                                                                                                                                                                                                                                                                        | Spends  | Approval |
| --------------------------------------------- | -----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------                                                                                                      | ------- | -------- |

`preview_fixups` is read only too: it runs an asset's saved fix-ups headless on a copy and returns the numbers before and after each. It spends nothing.

## Changes the project, spends nothing

| Tool                                                  | What it does                                                                                                                                                                                          | Spends  | Approval |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- | -------- |
| [`set_compression_presets`](#set_compression_presets) | Changes the per model texture size, data map size, codec and meshopt level                                                                                                                            | Nothing | No       |
| [`set_material_tweaks`](#set_material_tweaks)         | Changes an asset's tint, roughness, metalness, emissive strength and map codecs                                                                                                                       | Nothing | No       |
| [`set_avatar`](#set_avatar)                           | Changes a character asset's VRM meta, bone map overrides, material mode and confirmed springs                                                                                                         | Nothing | No       |
| [`set_ship_settings`](#set_ship_settings)             | Changes a website's Ship style, texture format, quality, per asset choice and file name, and the site folders                                                                                         | Nothing | No       |
| `set_bake_settings`                                   | Changes the lightmap size (256 to 4096), samples (1 to 4096) and denoise                                                                                                                              | Nothing | No       |
| `set_skip_bake`                                       | Sets one or more assets to skip the bake, or join it again                                                                                                                                            | Nothing | No       |
| `set_pad_edges`                                       | Pads an asset's texture edges before the textures shrink, or stops                                                                                                                                    | Nothing | No       |
| [`set_image_backend`](#set_image_backend)             | Sets the Lab's backend per kind of job and the Image API's quality. The monthly cap and the key are only ever set in the app                                                                          | Nothing | No       |
| `set_real_size`                                       | Sets or clears the real size a model is scaled to                                                                                                                                                     | Nothing | No       |
| `add_fixups`                                          | Adds fix-ups to an asset, Split into parts (`split-parts`) and Rotate (`rotate` with degrees `x`, `y`, `z`) among them; one of a kind already there is replaced                                       | Nothing | No       |
| [`set_parts`](#set_parts)                             | Renames, joins, unjoins, hides or shows an asset's parts                                                                                                                                              | Nothing | No       |
| [`assemble_prop`](#assemble_prop)                     | Joins part models into one prop, a node per part under one root, as the next version of an asset                                                                                                      | Nothing | No       |
| [`bake_back`](#bake_back)                             | Bakes the texture back onto a split that lost it, headless, as the asset's next version                                                                                                               | Nothing | No       |
| `remove_fixups`                                       | Removes fix-ups by kind, or all of them                                                                                                                                                               | Nothing | No       |
| `update_starters`                                     | Brings outdated or missing starter scripts up to date, as the Update banner does                                                                                                                      | Nothing | No       |
| [`set_layout`](#set_layout)                           | Changes placements, camera stops, desk, object states, explode tracks, paths and clips                                                                                                                | Nothing | No       |
| [`pick_concepts`](#pick_concepts)                     | Puts Lab concepts on the Board at Refs                                                                                                                                                                | Nothing | No       |
| `star_concepts`                                       | Stars or unstars Lab concepts                                                                                                                                                                         | Nothing | No       |
| [`set_style_board`](#set_style_board)                 | Changes a project's style board: the line, the palette and the images, copied into `style/`                                                                                                           | Nothing | No       |
| [`set_ref_views`](#set_ref_views)                     | Sets the view of every ref of an asset: front, left, back, right or other                                                                                                                             | Nothing | No       |
| `use_ref_candidate`                                   | Takes a drawn view into its slot (the ref it replaces moves to `refs/originals`), or a clean copy into the place of the ref it cleans (the original moves there too)                                  | Nothing | No       |
| [`accept_inbox_model`](#accept_inbox_model)           | Moves an inbox model into a project as a new asset version                                                                                                                                            | Nothing | No       |
| [`discard_inbox_model`](#discard_inbox_model)         | Moves an inbox model aside into `inbox/.discarded/` instead of deleting it                                                                                                                            | Nothing | No       |
| [`accept_inbox_models`](#accept_inbox_models)         | Accepts several inbox models in one call, each answering on its own                                                                                                                                   | Nothing | No       |
| [`restore_inbox_model`](#restore_inbox_model)         | Puts a discarded inbox model back into the inbox, aimed at the asset it was going to                                                                                                                  | Nothing | No       |
| [`attach_download`](#attach_download)                 | Sends a download from `list_handoffs` to the inbox for an asset                                                                                                                                       | Nothing | No       |
| [`add_model_file`](#add_model_file)                   | Adds a model file you fixed yourself as an asset's next version, through the inbox                                                                                                                    | Nothing | No       |
| `dismiss_download`                                    | Leaves a download from `list_handoffs` alone and never asks about it again                                                                                                                            | Nothing | No       |
| `copy_asset`                                          | Copies an asset with its refs and versions to another project                                                                                                                                         | Nothing | No       |
| `remove_assets`                                       | Takes up to 50 assets off the Board into the workspace trash, stopping their waiting sends                                                                                                            | Nothing | No       |
| `restore_asset`                                       | Puts a removed asset back from the workspace trash with its files, place and settings                                                                                                                 | Nothing | No       |
| `import_game_models`                                  | Imports models from the game repo, each as an asset whose version 1 is the repo file, and asks for their previews                                                                                     | Nothing | No       |
| `set_game_settings`                                   | Changes a game's format, model folder, export (dropping normal and metal maps too), bake, folder budgets, each asset's own folder (`assetFolders`) and the game repo (`repo`, a checkout or worktree) | Nothing | No       |
| `add_built_asset`                                     | Adds a model a Blender script in the project makes, and builds it headless                                                                                                                            | Nothing | No       |
| `rebuild_built_asset`                                 | Builds one built asset again, or every new or stale one                                                                                                                                               | Nothing | No       |
| `render_thumbnail`                                    | Renders a studio thumbnail of a model with headless Blender and returns the PNG's path                                                                                                                | Nothing | No       |
| `start_page_preview`                                  | Starts the site dev server the Page view shows, or reuses the running one                                                                                                                             | Nothing | No       |
| `stop_page_preview`                                   | Stops that server, whoever started it                                                                                                                                                                 | Nothing | No       |
| `cancel_run`                                          | Cancels a queued or running run. A run waiting for approval is cancelled too                                                                                                                          | Nothing | No       |
| [`add_to_library`](#add_to_library)                   | Copies an asset's newest model into the library                                                                                                                                                       | Nothing | No       |
| [`add_many_to_library`](#add_many_to_library)         | Copies up to 200 assets' newest models into the library in one call, each answering on its own                                                                                                        | Nothing | No       |
| [`use_from_library`](#use_from_library)               | Copies a library model into a project as a new asset                                                                                                                                                  | Nothing | No       |
| [`update_from_library`](#update_from_library)         | Adds a newer library version to an asset as its next version, or returns the plan                                                                                                                     | Nothing | No       |
| [`tag_library_entries`](#tag_library_entries)         | Puts tags on and takes tags off several library entries at once                                                                                                                                       | Nothing | No       |
| [`rename_library_entry`](#rename_library_entry)       | Renames a library entry; its id stays                                                                                                                                                                 | Nothing | No       |
| [`set_library_collection`](#set_library_collection)   | Creates, fills, empties, renames or deletes a library collection                                                                                                                                      | Nothing | No       |
| [`remove_from_library`](#remove_from_library)         | Moves library entries to `library/.removed`; the copies in projects stay                                                                                                                              | Nothing | No       |
| [`restore_to_library`](#restore_to_library)           | Puts removed library entries back, undoing a merge for a merged one                                                                                                                                   | Nothing | No       |
| [`merge_library_entries`](#merge_library_entries)     | Merges one library entry into another and points project assets along                                                                                                                                 | Nothing | No       |
| [`export_pack`](#export_pack)                         | Writes library models as one pack (OBJ or GLB, README, manifest, contact sheet, zip) inside the workspace, each checked in headless Blender                                                           |
| [`decide_pack`](#decide_pack)                         | Raises a pack model's budget, keeps a warning, exports it again, zips, stops or goes on                                                                                                               |
| [`import_animations`](#import_animations)             | Imports Mixamo FBX files or pack zips into the animation library, skeleton only                                                                                                                       | Nothing | No       |
| [`save_model_clips`](#save_model_clips)               | Saves a rigged model's clips to the animation library as a pack                                                                                                                                       | Nothing | No       |
| [`use_animations`](#use_animations)                   | Bakes library clips onto a rigged model as its next version, after a review                                                                                                                           | Nothing | No       |
| [`get_retarget_review`](#get_retarget_review)         | A model's last retarget review and the library clips baked into it                                                                                                                                    | Nothing | No       |

## Spends credits, Codex usage or dollars, or needs approval

| Tool                                          | What it does                                                                                                                                                                                                                          | Spends                                                          | Approval                                                                                                   |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| [`run_pipeline`](#run_pipeline)               | Runs a project's pipeline, optionally from a step or build only                                                                                                                                                                       | Nothing                                                         | At the Ship step, when Ask before shipping is on or the project is a game                                  |
| [`send_to_3d`](#send_to_3d)                   | Sends an asset's refs to Tripo or Meshy, or writes a By hand folder                                                                                                                                                                   | Credits for Tripo and Meshy. By hand spends nothing             | Always, for Tripo and Meshy                                                                                |
| `send_batch_to_3d`                            | Sends up to 24 assets at once with the same provider, with each asset's weak ref warnings and a modelWarning when it has a model                                                                                                      | Credits for Tripo and Meshy                                     | One approval for the whole batch                                                                           |
| `redo_inbox_model`                            | Sends an inbox model's asset again, prefilled from its send                                                                                                                                                                           | Credits for Tripo and Meshy                                     | Always, for Tripo and Meshy                                                                                |
| `redo_inbox_models`                           | Redoes several inbox models from one project as one batch                                                                                                                                                                             | Credits for Tripo and Meshy                                     | One approval for the whole batch                                                                           |
| [`split_with_provider`](#split_with_provider) | Splits an asset's model into parts on Tripo (segmentation) or Meshy (Auto Split)                                                                                                                                                      | Credits for Tripo and Meshy                                     | Always                                                                                                     |
| [`build_avatar`](#build_avatar)               | Builds a character asset's VRM 1.0 avatar into the project's `exports/<asset>.vrm`, freeing cloth and hair from the arms and normalising its rest pose headless first when it needs it                                                | Nothing                                                         | When the weights or the rest pose change: a person approves the before and after sheets in the Avatar view |
| `start_concepts`                              | Starts concept image jobs in the Lab, with the project style unless `useStyle` is false. `backend` picks Codex or OpenAI Images for this request                                                                                      | Codex usage, or dollars on OpenAI Images within the monthly cap | OpenAI Images only, when an agent asks                                                                     |
| `make_object_refs`                            | Makes one isolated image per object of a concept, with the project style unless `useStyle` is false. `backend` picks Codex or OpenAI Images for this request                                                                          | Codex usage, or dollars on OpenAI Images within the monthly cap | OpenAI Images only, when an agent asks                                                                     |
| `make_other_views`                            | Draws an asset's missing views (or the named ones) from its front ref, 1 to 4 candidates each, with the project style unless `useStyle` is false and `notes` in every prompt. `backend` picks Codex or OpenAI Images for this request | Codex usage, or dollars on OpenAI Images within the monthly cap | OpenAI Images only, when an agent asks                                                                     |
| `clean_ref_background`                        | Makes a copy of one ref with the object kept and the clutter cleared, on studio grey or transparent. `backend` picks Codex or OpenAI Images for this request                                                                          | Codex usage, or dollars on OpenAI Images within the monthly cap | OpenAI Images only, when an agent asks                                                                     |
| `retry_concept_jobs`                          | Runs failed or cancelled Lab jobs again, including jobs the app closed on. `include: "held"` or `"all"` runs the jobs waiting out a rate or usage limit now too                                                                       | Codex usage, or dollars on OpenAI Images within the monthly cap | OpenAI Images only, when an agent asks                                                                     |
| `retry_usage_limited_jobs`                    | Runs the Lab jobs waiting for the Codex usage limit now, but only when a fresh read says the limit has lifted; otherwise says until when it holds                                                                                     | Codex usage                                                     | No                                                                                                         |

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

| Parameter     | Type    | Required | Notes                                                                                                                                |
| ------------- | ------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `project`     | string  | yes      | The project id.                                                                                                                      |
| `from`        | string  | no       | A step id. It runs even when cached, and so does everything after it.                                                                |
| `force`       | boolean | no       | Ignore the cache and run every step.                                                                                                 |
| `buildOnly`   | boolean | no       | Stop before Ship: the run builds everything and writes nothing to the site.                                                          |
| `waitSeconds` | number  | no       | How long to wait before answering. 20 by default, 120 at most.                                                                       |
| `repo`        | string  | no       | A game project only: an absolute path to a git checkout or worktree that this one run ships into instead of the project's game repo. |

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

Sends an asset's refs to 3D. The images go by their views, front first, and refs tagged other stay home (see [Refs and views](/docs/views/board/#refs-and-views)). The answer carries `refWarnings`: weak refs found before anything is spent (small, cropped, busy background, one view with multi-view on, a side or the back that looks different from the front), each with its fixes. They never stop the send. `modelWarning` says when the asset already has a model, for example one that came from the library: the send then makes its next version and the library entry is not changed (null for a first model). `tripo` and `meshy` spend credits, so the send always waits for you to approve it in the Maquette app and answers `waiting for approval`. Follow it with `get_provider_job`. `by-hand` spends nothing: it writes a handoff folder with the images and a settings card, and the asset waits for a person to bring the GLB back. A by hand Tripo send takes `mesh` and `rig`, the Tripo Studio tab and rig the card asks for; more than the front view goes as Multiview to 3D, each image in the slot of its view. `send_batch_to_3d` takes both for the batch and per item, and the redo tools take them too, by default as the earlier send had them.

| Parameter   | Type     | Required | Notes                                                                                                                                           |
| ----------- | -------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `project`   | string   | yes      | The project id.                                                                                                                                 |
| `asset`     | string   | yes      | The asset id, as `list_assets` returns it.                                                                                                      |
| `provider`  | string   | yes      | `tripo`, `meshy` or `by-hand`.                                                                                                                  |
| `service`   | string   | no       | By hand only: whose web app the settings card is for, `tripo` by default.                                                                       |
| `variant`   | string   | no       | `prop` or `character`. By default from the asset's kind.                                                                                        |
| `images`    | string[] | no       | Images relative to the project folder, at most 4. By default the asset's view slots. Tagged images go in view order.                            |
| `overrides` | object   | no       | Preset fields for this send only, such as `{"faceLimit": 20000}`. An unknown name is refused with the valid list.                               |
| `prompt`    | string   | no       | Meshy only: text that guides the texture, at most 800 characters.                                                                               |
| `mesh`      | string   | no       | By hand on Tripo only: `hd-model` or `smart-mesh` (Smart Mesh (P2.0)). By default Smart Mesh for a P series preset.                             |
| `rig`       | string   | no       | By hand on Tripo only: `biped` (Mixamo skeleton), `quadruped`, `multi-legged` or `none`. By default biped for a character that rigs, else none. |

```json
{ "project": "moon-island", "asset": "far-spire", "provider": "tripo" }
```

### set_ref_views

`spends nothing`

Sets the view of every ref of an asset, as the refs sheet does. At most one ref per front, left, back and right; other refs stay with the asset but never go as views, and a ref left out is no longer the asset's (its file stays in `refs/`). Everything is checked before anything is saved, and the app updates live. `get_ref_views` reads them back, with `tagged` false while the views are read from the refs' order.

| Parameter | Type     | Required | Notes                                                                                   |
| --------- | -------- | -------- | --------------------------------------------------------------------------------------- |
| `project` | string   | yes      | The project id.                                                                         |
| `asset`   | string   | yes      | The asset id.                                                                           |
| `refs`    | object[] | yes      | `{ file, view }` for each ref, `view` one of `front`, `left`, `back`, `right`, `other`. |

```json
{
  "project": "frostveil",
  "asset": "frost-lantern",
  "refs": [
    { "file": "refs/frost_lantern.png", "view": "front" },
    { "file": "refs/frost_lantern_side.png", "view": "left" },
    { "file": "refs/lantern_kit_crop.png", "view": "other" }
  ]
}
```

`make_other_views` (`{ project, asset, views?, count?, useStyle?, notes? }`) draws the empty slots by default, 2 candidates each, from the front with the front attached; naming a filled view draws it again, and the front is never drawn. `notes` (one line, at most 400 characters, for example "the sword in the left hand") goes into every view's prompt as extra instructions, so a retry keeps it. `clean_ref_background` (`{ project, asset, file, background?, pad?, extend? }`) cleans one ref; `background` is `studio-grey` (default) or `transparent`, and nothing the frame cut off is drawn unless `extend` is true. Both are Lab jobs on the Codex lane, like `start_concepts`, in a direction named "<asset> views" or "<asset> clean". `use_ref_candidate` (`{ project, asset, file, view?, replace? }`) then takes one candidate in; nothing reaches the refs before that. A drawn view for a filled slot replaces the ref there: it leaves the asset's refs and its file moves to `refs/originals`, unless another asset shows it or a send lists it, in which case the file stays where it is and only this asset lets it go. `replace: false` keeps the old ref with the asset as Other. The answer's `replaced` names the old file, its view and where it went (`movedTo`) or why it stayed (`keptBecause`).

### set_image_backend

`changes the workspace settings`

`get_image_backend` reads how the Lab makes images: Codex CLI on the Codex subscription, or OpenAI's Image API paid per image from the OpenAI key saved in the app. Every kind of job starts on Codex. `set_image_backend` (`{ concepts?, objectRefs?, otherViews?, cleanBackground?, quality? }`) changes the backends and quality in Settings > Image generation; fields left out keep their value and `quality` is `low`, `medium` or `high`. Settings follows the change live. The monthly cap is not an agent's to change: only you set it, in Settings, and a request that carries `monthlyCapUsd` is refused with nothing changed. `get_image_backend` still reads the cap. The OpenAI key is never set or read over MCP.

`start_concepts`, `make_object_refs`, `make_other_views` and `clean_ref_background` take `backend` (`codex` or `openai-images`) to override the setting for one request, and every job they return names its `backend`. A round an agent asks for on the Image API waits for you: its jobs sit in state `approval`, the app shows Make N images with OpenAI Images? with the estimate and how the month stands, and the answer is `waiting for approval` with `approval.estimateUsd`. The agent polls `get_concepts` and must not start the round again. Approve queues the jobs; Deny ends them cancelled with nothing sent. `retry_concept_jobs` on an Image API job waits the same way. A round you start in the Lab never asks. Either way, a batch that would take the month's spend past the cap is refused before anything is made, and every image is written to `credits.jsonl` in dollars, marked as an agent's when it was. The estimate is what an image at the chosen quality has cost lately (the average of the last 20 measured images in the ledger, once there are 3), else the price list's guess.

When OpenAI refuses an image on safety grounds (a 400 from its safety system), the job fails with `refused: true` and a message that gives OpenAI's reason, says nothing was charged and asks to reword the prompt; nothing is written to the ledger. A retry sends the same prompt, so it is likely refused again.

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

### get_avatar

`read only`

Reads a character asset's VRM avatar, as the [Avatar view](/docs/views/avatar/) shows it: the VRM readiness of the version (the 55 humanoid bones with the joint each maps to and the naming it matched by, the 15 required, the naming report (the convention the joints matched, ambiguous matches and whether the map is confident), the skin's joints to pick from, the rest pose measures and the findings, blockers first), the spec (the VRM meta, the bone map overrides and the material mode, taken from Settings > Avatar defaults until it is first saved), whether the rest pose needs normalising, the last normalised rest pose with its before and after sheets and its state (`pending`, `approved` or `denied`), `springs`, the last export and `waitingApproval`. `springs.ready` says the model the VRM is written from is ready (its rest pose is VRM's, or its new one is approved); `springs.found` lists the parts that could swing a build found on it, each with an `id` such as `hair-1`, a `kind` (`hair`, `cloth` or `accessory`), the `rootBone` it hangs from, its centre `line`, `length`, `vertices`, `confidence` (0 to 1), the `joints` a chain gets by its length and its `color` on `springs.found.sheet`; `springs.chained` is the last chain step with its sheets and state. The confirmed springs are `spec.springs`.

| Parameter | Type   | Required | Notes                                      |
| --------- | ------ | -------- | ------------------------------------------ |
| `project` | string | yes      | The project id.                            |
| `asset`   | string | yes      | The asset id or name.                      |
| `version` | number | no       | The version to read; the newest otherwise. |

### set_avatar

`changes the project`

Changes a character asset's avatar spec in the project's `avatar.json`, as the Avatar view's form does. Only what you give changes.

| Parameter    | Type    | Required | Notes                                                                                                                                                 |
| ------------ | ------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `project`    | string  | yes      | The project id.                                                                                                                                       |
| `asset`      | string  | yes      | The asset id or name.                                                                                                                                 |
| `reset`      | boolean | no       | Start again from the workspace's avatar defaults first.                                                                                               |
| `meta`       | object  | no       | VRM 1.0 meta fields by name (`name`, `authors`, `commercialUsage`, `allowRedistribution` and the rest); `null` puts one back to its default.          |
| `bones`      | object  | no       | A VRM bone name to the joint it maps to, or `null` to leave the bone unmapped.                                                                        |
| `clearBones` | array   | no       | Bones whose override is dropped, so they map by name again.                                                                                           |
| `material`   | string  | no       | `mtoon-soft` (toon shading from the base color, no outline) or `pbr`.                                                                                 |
| `highlights` | string  | no       | With `mtoon-soft`: `soften` (the default) tones down strong highlights baked into dark material on a copy of the texture; `keep` leaves it.           |
| `bridges`    | string  | no       | `hide` (the default) takes the faces fused between an arm and the body, which stretch when it lifts, out of the reweighted model; `keep` leaves them. |
| `springs`    | array   | no       | The springs to confirm, replacing the ones before, or `"none"` for no springs; `[]` only clears the picks. See below.                                 |

Each spring names a candidate from `get_avatar`'s `springs.found` by `id`, and takes the rest from it: `{ "id": "hair-1" }`. It can add `joints` (1 to 16), `preset` (`soft` for hair, `heavy` for cloth, `stiff` for small parts; by kind otherwise) and the swing itself: `stiffness` (0 to 4), `gravityPower` (0 to 2), `dragForce` (0 to 1) and `hitRadius` (0 to 0.5 metres). A small piece (`small: true`, listed after the other parts as `charm-1`, `charm-2`: a charm on a belt or a jacket, a short chain, an earring close to the body) keeps its short chain of 2 to 4 joints unless `joints` is given, and swings `stiff` as an accessory. A part the finder did not list takes `kind`, `rootBone` and `line` too. Nothing is confirmed on its own: confirm only the parts a person or you judged from the candidates sheet (a rigid brim looks like hair to the finder), and say which.

`"springs": "none"` confirms that the VRM carries no springs. An empty list is not the same: it clears the picks, and when the last export had springs, the next build answers `candidates waiting for a pick` instead of writing the file without them.

A build needs a name and at least one author. The licence URL is always VRM's own; put another licence in `otherLicenseUrl`.

```json
{
  "project": "pets",
  "asset": "hero",
  "meta": { "authors": ["Me"], "commercialUsage": "personalProfit" },
  "material": "mtoon-soft"
}
```

### build_avatar

`needs approval when the weights, the rest pose or the spring chains change`

Builds a character asset's VRM 1.0 avatar into the project's `exports/<asset>.vrm`, over the last one. It is a derived export: the asset gets no new version. When the rest pose is not VRM's T-pose yet (facing +Z, arms level, on the ground, centred), Blender first normalises it headless on a copy and draws before and after pose sheets, with a last row of both hands up close. When the readiness finds cloth or hair weighted to the arms, `reweight: "auto"` (the default) frees it before that: Blender moves its weight from the arm bones to the trunk on a copy, keeping the arms, sleeves and gloves, and draws before and after sheets. Faces fused between an arm and the body far from the shoulder, which stretch whatever the weights, are named in `reweight.bridges` and taken out of the model unless `set_avatar` set `bridges: "keep"` (`reweight.hiddenFaces` says how many). New weights and a changed rest pose each need a person's yes, so the call answers `waiting for approval` with `step` (`reweight` first, then `rest pose`) and the sheet paths, and the app shows the request; a person approves or denies it in the Avatar view. A no to the weights lifts the arms from the model as it is. No tool approves either. Call `get_avatar` or `build_avatar` again later: the normalised result is cached, so Blender does not run again, and once approved the call writes the file and answers `written` with its path, size, what it still lacks, `report` (the path of `exports/<asset>.vrm.report.json`, written beside it: the source version and its hash, the readiness before and after, the bone map with each bone's source, the meta, what the reweight moved when one was applied, and with MToon how much of each base color texture is baked highlights and whether a softened copy was written), `highlights` (the same, for the answer) and `roundTrip` (the written file read back and checked: `passed`, `problems` and the bones read; a failure is also a blocker in what it still lacks). `denied` means the person turned the pose down; build again with other options once they say what to change.

`springs: "auto"` (the default) adds the confirmed springs (`set_avatar` `springs`) once the rest pose is approved: Blender adds a chain of joints along each part, weights the part to it, measures colliders from the body and draws a sheet of the parts and a sway test, on a copy. The chains need a person's yes too: the call answers `waiting for approval` with `step` `springs`, the sheets and each chain. After the yes the file carries `VRMC_springBone` (a spring per part with its preset or tuned values, the colliders and their groups), the answer's `springs` counts them and the report lists them; after a no it is written without springs. With none confirmed, a build looks for the parts that could swing (once per model) so `get_avatar` lists them; when the last export carried springs it then answers `candidates waiting for a pick` with the candidates and writes nothing, until `set_avatar` confirms some or `"none"`. With `"none"` confirmed it writes no springs. Before chaining, a build holds each confirmed spring to the parts found on the model now: one whose part the finder names differently (after a new reweight or rest pose) is carried over to the part on its line, keeping its kind, joints and preset, saved, and listed in the answer's `carried` (`from`, `to`, `rootBone`, `overlap`). When no part matches a spring clearly, the call answers `springs no longer match` with `stale` (the ids) and the candidates, and writes nothing: pick again. `springs: "off"` writes no springs.

| Parameter  | Type    | Required | Notes                                                                                                      |
| ---------- | ------- | -------- | ---------------------------------------------------------------------------------------------------------- |
| `project`  | string  | yes      | The project id.                                                                                            |
| `asset`    | string  | yes      | The asset id or name.                                                                                      |
| `version`  | number  | no       | The version to build from; the newest otherwise.                                                           |
| `dryRun`   | boolean | no       | Return the plan: what it reads, whether it normalises, what is cached and where it writes.                 |
| `elbows`   | string  | no       | `straighten` (the default) levels the lower arms too; `keep` leaves bent elbows bent.                      |
| `fingers`  | string  | no       | `straighten` (the default) lays the fingers straight as far as the mesh holds; `keep` leaves them curled.  |
| `reweight` | string  | no       | `auto` (the default) frees cloth and hair from the arms first when the readiness finds it; `off` skips it. |
| `springs`  | string  | no       | `auto` (the default) adds the confirmed springs, after their chains are approved; `off` writes none.       |

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

### set_parts

`changes the project`

Renames, joins, hides or shows an asset's parts, as the Inspector's Parts tab does. Parts are named by their id (the name in the model, such as `part_2`) or the name they ship as; see `get_parts`. Renames and joins ship in the next build, as the export's node names, which object states and explode tracks pick parts by. Hiding only draws a part as a ghost in the Inspector. Nothing is written when any part of the change is refused. Returns the parts like `get_parts`.

| Parameter | Type       | Required | Notes                                                                         |
| --------- | ---------- | -------- | ----------------------------------------------------------------------------- |
| `project` | string     | yes      | The project id.                                                               |
| `asset`   | string     | yes      | The asset id or name.                                                         |
| `rename`  | object     | no       | Part to its new name. Unique, at most 64 characters; the id forgets a rename. |
| `merge`   | string[][] | no       | Groups of two or more parts joined into one, named after the first.           |
| `unmerge` | string[]   | no       | Parts whose join is undone.                                                   |
| `hide`    | string[]   | no       | Parts drawn as a ghost in the Inspector. They still ship.                     |
| `show`    | string[]   | no       | Hidden parts drawn again.                                                     |

A model that is one mesh lists one part. Split it first: `add_fixups` with `{ "op": "split-parts", "by": "loose", "mergeUnder": 0.01 }` (`by` is `loose` or `material`, `mergeUnder` 0 to 0.2), then `preview_fixups` or `get_parts` runs it headless on a copy. When the pieces are welded into one shell, `split_with_provider` lets Tripo or Meshy find the parts.

### split_with_provider

`costs credits` `needs approval`

Splits an asset's model into parts on Tripo or Meshy, like the split sheet in the Parts tab. It always waits for you to approve it in the Maquette app and answers `waiting for approval`; follow it with `get_provider_job`. The split lands in the inbox and `accept_inbox_model` makes it the asset's next version; the version it came from stays. Tripo's parts arrive as `tripo_part_0`, `tripo_part_1` and on (name them with `set_parts`). Meshy splits only models it made with Meshy 6 or 7 and drops the texture, so once the split is accepted the texture bakes back from the version it came from in headless Blender, free, as the version after.

| Parameter             | Type     | Required | Notes                                                                                     |
| --------------------- | -------- | -------- | ----------------------------------------------------------------------------------------- |
| `project`             | string   | yes      | The project id.                                                                           |
| `asset`               | string   | yes      | The asset id or name.                                                                     |
| `provider`            | string   | yes      | `tripo` (about 40 credits) or `meshy` (about 10 credits).                                 |
| `version`             | number   | no       | The version to split. By default the current one.                                         |
| `item`                | string   | no       | An inbox model of the asset (its id in `list_inbox`) instead of a version.                |
| `model`               | string   | no       | Tripo: `geometry` (default) or `semantic` (beta).                                         |
| `granularity`         | string   | no       | Tripo semantic: `simple`, `balanced` (default) or `detailed`.                             |
| `splitByConnectivity` | boolean  | no       | Tripo semantic: loose pieces become parts too. True by default.                           |
| `complete`            | string   | no       | Tripo: `ai-completion` or `quick-cap` runs part completion after the split, more credits. |
| `mode`                | string   | no       | Meshy: `auto` (default), `by-parts` or `by-color`.                                        |
| `parts`               | string[] | no       | Meshy by parts or by colour: 1 to 10 names to look for.                                   |

```json
{
  "project": "frostveil",
  "asset": "frost-lantern",
  "provider": "meshy",
  "mode": "by-parts",
  "parts": ["lid", "cage", "crystal"]
}
```

### bake_back

`changes the project`

Bakes the texture back onto a split that lost it, like Bake back in the Parts tab. It runs when the app offers the bake (a Tripo split that came back grey) or when an earlier bake failed; a Meshy split bakes back on its own once accepted. The texture of the version the split came from bakes headless in Blender onto the split version, and the result lands as the asset's next version; the untextured version stays. It waits for Blender and answers with the job, the bake's state (`done` or `failed`), the new version and a message.

| Parameter | Type   | Required | Notes                                                                    |
| --------- | ------ | -------- | ------------------------------------------------------------------------ |
| `project` | string | yes      | The project id.                                                          |
| `asset`   | string | yes      | The asset id or name.                                                    |
| `version` | number | no       | The split version the texture bakes onto. By default the newest offered. |

```json
{ "project": "frostveil", "asset": "frost-lantern", "version": 2 }
```

### assemble_prop

`changes the project`

Joins part models (each an asset with its own versions) into one prop, headless in Blender: one GLB with a node per part under one root named after the prop. It lands as the next version of `target`, or of an asset named after the prop, made when there is none. The recipe is kept, so assembling again picks up a part's newer version. Answers with the asset, the version, the root and each node with where it sits.

| Parameter | Type     | Required | Notes                                                                                                                                                                                                                                                                                    |
| --------- | -------- | -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `project` | string   | yes      | The project id.                                                                                                                                                                                                                                                                          |
| `name`    | string   | yes      | The prop's name: its root node, and the new asset's name.                                                                                                                                                                                                                                |
| `target`  | string   | no       | The asset it becomes the next version of.                                                                                                                                                                                                                                                |
| `parts`   | object[] | yes      | 2 to 16 parts in node order, each `{ asset, version?, name, pivot?, at?, turn? }`: `pivot` is `bottom-centre` (default), `centre`, `left-hinge`, `right-hinge` or `origin`; `at` is where the pivot goes in metres, Y up, by default where its model put it; `turn` is degrees about up. |

```json
{
  "project": "frostveil",
  "name": "sigil_gate",
  "parts": [
    { "asset": "sigil-gate-frame", "name": "frame" },
    { "asset": "sigil-gate-door", "name": "door_leaf", "pivot": "left-hinge", "at": [-0.6, 0, 0] },
    { "asset": "sigil-gate-steps", "name": "steps" }
  ]
}
```

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

### set_style_board

`changes the project`

Changes the project's style board, the look every new Lab direction and object ref carries. Only the fields given change. The board lives in `style.json` at the project root, its images copied into `style/`; each change that differs bumps `revision`. It spends nothing, and the Lab updates live.

| Parameter | Type     | Required | Notes                                                                |
| --------- | -------- | -------- | -------------------------------------------------------------------- |
| `project` | string   | yes      | The project id.                                                      |
| `line`    | string   | no       | One style line, at most 200 characters. `""` clears it.              |
| `palette` | string[] | no       | Up to 6 colours as `#rrggbb` or `#rgb`. `[]` clears the palette.     |
| `images`  | string[] | no       | The whole new list, at most 4. `[]` takes every image off the board. |

Each image is a board image as `get_style_board` lists it (`style/...`), a concept or ref of the project as `get_concepts` lists it (`concepts/<direction>/<file>.png`, `refs/<file>.png`), or an absolute path inside the workspace. A path outside the workspace, or one that climbs out of the project, is refused. Each is copied into `style/`, never linked.

`start_concepts` and `make_object_refs` carry the board by default: the style line and palette go after the brief, and the board's images are attached after the brief's own refs, so the brief leads. `useStyle: false` starts them without it. Their results, and the directions and jobs in `get_concepts`, say `styleRevision` (null when the board was off or empty), and a retried job carries the same board it started with.

### accept_inbox_model

`changes the project`

Moves a model from the inbox into a project as a new asset version on the Board, and renders its thumbnail headless. Any By hand send of that asset still waiting for its model closes as done, so `list_handoffs` no longer lists it; a Tripo or Meshy send runs on.

| Parameter | Type   | Required | Notes                                                |
| --------- | ------ | -------- | ---------------------------------------------------- |
| `id`      | string | yes      | The inbox item id, its file name, from `list_inbox`. |
| `project` | string | yes      | The project that gets the model.                     |
| `asset`   | string | no       | An existing asset id to add a version to.            |
| `name`    | string | no       | The asset name, when a new asset is made.            |

Without `asset` it goes to the asset the model was sent for, or the one a download was attached to (`aimedAt` in `list_inbox`).

### discard_inbox_model

`changes the inbox`

Throws away an inbox model, such as the losing version of a redo or a bad download. The file is not deleted: it moves into `inbox/.discarded/` in the workspace (a name already there gets a number), and `restore_inbox_model` puts it back. Its cached checks, picked kind and aim are forgotten. A send that was waiting on this model in the Inbox ends as cancelled with the note `Discarded from the Inbox`; what the send already cost stays as it was in the credits ledger, and an asset's earlier versions are untouched. It answers with the `file` it moved to and `closedSend`, the id of that send or null.

| Parameter | Type   | Required | Notes                                                |
| --------- | ------ | -------- | ---------------------------------------------------- |
| `id`      | string | yes      | The inbox item id, its file name, from `list_inbox`. |

### accept_inbox_models

`changes the project`

Accepts several inbox models in one call, each as `accept_inbox_model` does: it goes to the asset it was sent for or attached to, as that asset's next version. Without `project` each model goes to the project of that asset, and a model that goes nowhere fails on its own; with `project` every model goes there. Every model answers on its own, so an unreadable file does not stop the rest. The answer counts `accepted` and `failed`, and `results` lists each id with `ok`, then its `project`, `asset`, `version` and `file`, or its `error`. A By hand send of an accepted asset still waiting closes as done. It spends nothing and needs no approval, like `accept_inbox_model`.

| Parameter | Type     | Required | Notes                                                                          |
| --------- | -------- | -------- | ------------------------------------------------------------------------------ |
| `ids`     | string[] | yes      | 1 to 200 inbox item ids from `list_inbox`.                                     |
| `project` | string   | no       | The project that gets every model. Left out, each goes to its asset's project. |

```json
{ "ids": ["lantern_post.glb", "bench_v2.glb"] }
```

### restore_inbox_model

`changes the inbox`

Puts a discarded model back into the inbox. It returns under the inbox name it had (a name taken since gets a number) and is aimed at the asset it was going to, so `accept_inbox_model` goes there again; the send its discard closed stays closed. It answers with its inbox `id` and `file`. `list_inbox` lists the discarded models under `discarded`, the most recent first, with their `name`, the inbox `id` they had, `bytes`, `discardedMs`, the `assetName` they were going to and `replacedBy`. It spends nothing and needs no approval.

| Parameter | Type   | Required | Notes                                                              |
| --------- | ------ | -------- | ------------------------------------------------------------------ |
| `name`    | string | yes      | The discarded model's file name, from `discarded` in `list_inbox`. |

`list_inbox` also gives every waiting model `nextVersion`, the version Accept would make of the asset it goes to, and `replacedBy`, the newer version of that asset made after the model arrived (a redo that was accepted), when there is one.

### list_handoffs

`read only`

The By hand sends waiting for a model made on the provider's site (`pending`: job, asset, provider, the export name `maq_<asset>_v<n>_<provider>` it waits for, the handoff folder, the front ref, about what the site charges) and the Downloads watcher (`watcher`, null when no watcher runs). Only a download named for its send goes into the inbox by itself. The rest wait for an answer:

- `needsLook`: downloads whose picture looks like a waiting send's, whose name is close to one, or that came for the only send waiting, however late. Each names the asset it looks like (`asset`, `assetName`, `job`) and why (`reason`, for example "Looks like Lantern post (88%)" or "Name close to Lantern post"). Nothing went in yet.
- `unmatched`: downloads it could not place.

Each of those carries `suggestions`: every waiting send ranked best first, with `score` (0 to 1), `picture` (how alike the download's headless front render and the send's front ref are, null until the render is in or without Blender), `name` (how close the file name is) and `image` (the ref compared). `picture` says where the render stands (`queued`, `ready`, `failed` or `skipped`) and `render` is its PNG. A suggestion is never attached by itself: check it, then call `attach_download`.

Answer each with `attach_download` or `dismiss_download`. `arrivals` lists every recent file with its state. Takes an optional `project`.

### attach_download

`changes the project`

Attaches a download from `needsLook` or `unmatched` to an asset: it goes into the inbox for that asset, as a download named for its send does. It is handed to the asset's newest waiting By hand send, or, when none waits, put in the inbox aimed at the asset (`list_inbox` shows it as `aimedAt`). The usual checks run there. It answers with the arrival, its `inboxFile` (the inbox id) and the `job` it went to, if any; `accept_inbox_model` with that id then goes to the asset by default and closes the asset's By hand sends still waiting. Look at a `needsLook` download before attaching it: the watcher never attaches one by itself, so the call is the check. The download stays where it is.

| Parameter | Type   | Required | Notes                                      |
| --------- | ------ | -------- | ------------------------------------------ |
| `arrival` | string | yes      | The download's `id`, from `list_handoffs`. |
| `project` | string | yes      | The project the asset is in.               |
| `asset`   | string | yes      | The asset id or name that gets the model.  |

### add_model_file

`changes the project`

Adds a model file you made or fixed yourself as an asset's next version. `path` is an absolute path to a `.glb`, `.gltf`, `.fbx` or `.obj`; a file inside the workspace's inbox or a project's `models` folder is refused, since it is there already. The file is copied, never moved: a `.glb` as it is, the others turned into a GLB by headless Blender first. It goes into the inbox for the asset as `attach_download` does: to the asset's newest waiting By hand send when one waits (accepting closes it), else aimed at the asset. The usual checks run there. It answers with the `inboxFile`, the `job` it went to, if any, and the asset; with `accept` true it is accepted at once and `accepted` carries the new version.

| Parameter | Type    | Required | Notes                                                  |
| --------- | ------- | -------- | ------------------------------------------------------ |
| `project` | string  | yes      | The project the asset is in.                           |
| `asset`   | string  | yes      | The asset id or name that gets the model.              |
| `path`    | string  | yes      | The absolute path to the model file.                   |
| `accept`  | boolean | no       | Accept it into the asset right away. False by default. |

```json
{ "project": "harbour", "asset": "lantern", "path": "/Users/me/Fixes/lantern.glb", "accept": true }
```

`dismiss_download` takes `arrival` and leaves the file alone for good (Not this one, or Not a Maquette model in the app): its send keeps waiting and the same file is never asked about again.

### get_credits

`read only`

Reads each provider's balance (a free call, skipped without a saved key), totals for the last 7 and 30 days with the share spent on redos, and the most recent paid sends from the local ledger. `openaiImages` has the Lab's OpenAI Image API dollars: `monthToDateUsd` (this local calendar month, every project), `capUsd`, `leftUsd`, `pendingUsd` (jobs queued or waiting for approval), `imagesThisMonth`, `quality`, `perImageUsd` (what an image has cost lately, the figure the cap check uses), the most recent image lines under `recent`, and `projectMonthUsd` when `project` is given.

| Parameter | Type    | Required | Notes                                                       |
| --------- | ------- | -------- | ----------------------------------------------------------- |
| `project` | string  | no       | Only this project's spends.                                 |
| `limit`   | number  | no       | How many recent spends to list. 20 by default, 200 at most. |
| `balance` | boolean | no       | Ask Tripo and Meshy for the balance. `true` by default.     |

### list_library

`read only`

Lists the models in the [Library](/docs/views/library/). Each entry is compact by default: id, name, kind, tags, newest version, version count, use count, triangles and what made it. With `full`, it also has the format, sizes, credits and the projects that use each. It returns one page, with `total` (the number that match) and `nextCursor` (null on the last page), and `collections` lists every collection with its id, name and count. With `project`, it also lists that project's assets that have an update waiting.

| Parameter    | Type    | Required | Notes                                                              |
| ------------ | ------- | -------- | ------------------------------------------------------------------ |
| `query`      | string  | no       | Matches a model's name, tag or project.                            |
| `tag`        | string  | no       | Only models with this tag.                                         |
| `project`    | string  | no       | Also list this project's assets with an update waiting.            |
| `collection` | string  | no       | Only models in this collection (its id).                           |
| `limit`      | number  | no       | Entries per page: 25 by default, 100 at most.                      |
| `cursor`     | string  | no       | The `nextCursor` of the previous page. Opaque: pass it back as is. |
| `full`       | boolean | no       | List every entry in full instead of compact. False by default.     |

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

### add_many_to_library

`changes the library`

Adds several assets of one project to the library in one call, each as `add_to_library` does. Every asset answers on its own, so one that fails (an unknown asset, an asset without a model) does not stop the rest. The answer counts `added` and `failed`, and `results` lists each asset with `ok`, then its `entry`, `version`, `added` and `tags`, or its `error`. With `collection`, every added entry joins that collection. It spends nothing and the app updates live.

With `dryRun`, it adds nothing and returns the plan: for every asset with a model (or only the ones in `assets`) its `state`, one of `new`, `next-version`, `same-file`, `name-clash`, `looks-like` (its picture scores at or over the look-alike threshold against an entry's) or `unreadable` (the model is missing or cut short, with the `reason`), the `entry` it meets and `looks`, how alike the two pictures are. The plan also has the suggested project `tag`.

| Parameter    | Type     | Required | Notes                                                                                                                                                                                             |
| ------------ | -------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `project`    | string   | yes      | The project id.                                                                                                                                                                                   |
| `assets`     | object[] | yes      | 1 to 200 items of `{ asset, name?, tags?, into? }`. `tags` on an item replace the shared `tags`; `into` (an entry id) adds the model as that entry's next version. May be left out with `dryRun`. |
| `tags`       | string[] | no       | Tags for every asset that names none of its own. Lowercased, at most 16, 32 characters each.                                                                                                      |
| `collection` | string   | no       | A collection id every added entry joins.                                                                                                                                                          |
| `dryRun`     | boolean  | no       | Return the plan and add nothing.                                                                                                                                                                  |

```json
{
  "project": "moon-island",
  "tags": ["furniture"],
  "assets": [{ "asset": "bench" }, { "asset": "lamp", "tags": ["lighting"] }]
}
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

### tag_library_entries

`changes the library`

Puts tags on and takes tags off several library entries at once. Every entry is checked first, so an unknown entry or one that would pass 16 tags changes nothing. It answers each entry with its tags and spends nothing.

| Parameter | Type     | Required | Notes                                      |
| --------- | -------- | -------- | ------------------------------------------ |
| `entries` | string[] | yes      | Entry ids, as `list_library` returns them. |
| `add`     | string[] | no       | Tags to put on every entry. Lowercased.    |
| `remove`  | string[] | no       | Tags to take off every entry.              |

```json
{ "entries": ["bench", "desk-lamp"], "add": ["study"], "remove": ["draft"] }
```

### rename_library_entry

`changes the library`

Renames a library entry. Its id (its folder) stays, so the projects that came from it still find it. It spends nothing.

| Parameter | Type   | Required | Notes                                |
| --------- | ------ | -------- | ------------------------------------ |
| `entry`   | string | yes      | The entry id.                        |
| `name`    | string | yes      | The new name, 80 characters at most. |

### set_library_collection

`changes the library`

Changes the library's collections: named groups of entries, kept in `library/collections.json`. An entry can sit in several. `create` makes one from `name` (and `entries`), `add` and `remove` put entries in and take them out, `rename` gives it a new `name`, and `delete` removes the collection only, never a model. Two collections cannot share a name. It answers every collection with its entry ids and spends nothing.

| Parameter    | Type     | Required | Notes                                            |
| ------------ | -------- | -------- | ------------------------------------------------ |
| `op`         | string   | yes      | `create`, `add`, `remove`, `rename` or `delete`. |
| `collection` | string   | no       | The collection id, for every op but `create`.    |
| `name`       | string   | no       | The name, for `create` and `rename`.             |
| `entries`    | string[] | no       | Entry ids, for `create`, `add` and `remove`.     |

```json
{ "op": "create", "name": "Study props", "entries": ["bench", "desk-lamp"] }
```

### remove_from_library

`changes the library`

Removes entries from the library. Their folders move to `library/.removed`, so `restore_to_library` brings them back. The copies in projects stay as they are, and collections keep the ids for a restore. An unknown entry removes nothing. It spends nothing.

| Parameter | Type     | Required | Notes          |
| --------- | -------- | -------- | -------------- |
| `entries` | string[] | yes      | The entry ids. |

### restore_to_library

`changes the library`

Puts removed entries back. An entry that `merge_library_entries` merged comes back by undoing the merge: the versions it added to the kept entry go, the kept entry's tags and refs come back, and the project assets it moved point back at it. An undo is refused, with the reason, once the kept entry has newer versions or a project uses a version the merge added. It answers the entries restored and the projects that changed, and spends nothing.

| Parameter | Type     | Required | Notes                  |
| --------- | -------- | -------- | ---------------------- |
| `entries` | string[] | yes      | The removed entry ids. |

### find_look_alikes

`read only`

Finds library entries that may be one model twice. A same-look pair's pictures (each entry's newest thumbnail) score at or over the threshold on outline, colours and proportions; a same-name pair has names equal once a trailing number is dropped. Each pair names the older entry as `a`, with both names and the scores when both pictures read. Pairs a person decided in the app are left out. It also returns `unseen` (entries with no readable picture), `merges` (the recent merges, newest first, each undone with `restore_to_library` on its merged entry) and the `settings`. It takes no parameters.

### merge_library_entries

`changes the library`

Merges one library entry into another. Every version of `merge` the kept entry lacks (by file) becomes the kept entry's next version, its refs come along and, with `copyTags`, its tags. Project assets that came from `merge` point at the kept entry's matching version, so every project keeps the model it uses. The merged entry then waits in `library/.removed`, and `restore_to_library` on it undoes the merge. It spends nothing.

| Parameter  | Type    | Required | Notes                                                                  |
| ---------- | ------- | -------- | ---------------------------------------------------------------------- |
| `keep`     | string  | yes      | The entry that stays.                                                  |
| `merge`    | string  | yes      | The entry that goes into it.                                           |
| `copyTags` | boolean | no       | Copy the merged entry's tags. The saved look-alike setting by default. |

```json
{ "keep": "desk-lamp", "merge": "desk-lamp-2" }
```

### export_pack

`writes files`

Writes a set of models as one pack for another engine or a person: library `entries`, a library `collection` or a `project`'s accepted models (give one). It starts from a `preset` (`list_packs`; `plain-obj`, the built in one, by default) with the `settings` fields you give laid over it. Blender runs headless on each model: the real size its project gave it, the pivot at the bottom centre, the kind's triangle budget (a rigged model is never decimated), textures as PNG up to the size asked, materials named `body`, then OBJ with its MTL, GLB, or both. Each written file is opened again in an empty scene and checked: triangles, height, where it stands, textures that did not load and clips left.

Mixamo clips are never shared. A model on a Mixamo rig (or with clips baked from the animation library) keeps its bones and loses its clips, and the README and manifest say so.

A pack an agent asks for is written inside the workspace only, to `exports/<name>` by default, with `README.txt`, `manifest.json` (names, versions, triangles, source and licence notes) and `contact-sheet.png`, and zipped next to the folder once every model passes. A warning or a failure waits for [`decide_pack`](#decide_pack). With `dryRun`, it starts nothing and answers the plan: the models, the biggest model per kind against its budget, about what the zip weighs and how many models lose Mixamo clips. It spends no credits.

| Parameter    | Type     | Required | Notes                                                                                                                                                                                                                                                                                    |
| ------------ | -------- | -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `entries`    | string[] | no       | Library entry ids, as `list_library` returns them.                                                                                                                                                                                                                                       |
| `collection` | string   | no       | A library collection id: every entry in it.                                                                                                                                                                                                                                              |
| `project`    | string   | no       | A project id: every accepted model.                                                                                                                                                                                                                                                      |
| `preset`     | string   | no       | A preset id. `plain-obj` by default.                                                                                                                                                                                                                                                     |
| `settings`   | object   | no       | Fields to change: `name`, `format` (`obj`, `glb` or `both`), `realSize`, `pivotBottom`, `oneMaterial`, `texture` (512 to 4096, or 0 to keep), `riggedGlb`, `compression` (`none` or `web`), `budgets` (`[{ kind, triangles }]`), `readme`, `manifest`, `contactSheet`, `refs` and `zip`. |
| `out`        | string   | no       | A folder inside the workspace. `exports/` by default.                                                                                                                                                                                                                                    |
| `dryRun`     | boolean  | no       | Answer the plan and start nothing.                                                                                                                                                                                                                                                       |

```json
{ "collection": "village-props", "settings": { "name": "village-props", "format": "both" } }
```

### get_pack

`read only`

Reads one export pack: its `state` (`running`, `waiting`, `zipping`, `done`, `stopped` or `failed`), each model's check (`queued`, `running`, `pass`, `warn`, `fail` or `kept`) with its triangles before and after, height, note and, when thin parts were lost, the budget to raise to (`suggest`), how many need you, and the folder and zip.

| Parameter | Type   | Required | Notes                                                   |
| --------- | ------ | -------- | ------------------------------------------------------- |
| `id`      | string | yes      | The pack id, as `export_pack` or `list_packs` gives it. |

### list_packs

`read only`

Lists the newest export packs (a pack Maquette closed on reads as `stopped`) and the pack presets with their settings.

### decide_pack

`writes files`

Answers what a pack asks for. `raise` (with `key` and `triangles`) exports a model again at a higher budget, `keep` (with `key`) lets a warned model into the pack, `again` (with `key`) exports a model again as it is. `zip-passing` zips the models that pass and leaves the rest out, `zip-all` zips every model with files. `stop` stops after the model running now and `resume` goes on with a stopped pack from the model it reached. It answers the pack as `get_pack` does.

| Parameter   | Type   | Required | Notes                                                                   |
| ----------- | ------ | -------- | ----------------------------------------------------------------------- |
| `id`        | string | yes      | The pack id.                                                            |
| `choice`    | string | yes      | `raise`, `keep`, `again`, `zip-passing`, `zip-all`, `stop` or `resume`. |
| `key`       | string | no       | The model's key in the pack, for `raise`, `keep` and `again`.           |
| `triangles` | number | no       | The new budget, for `raise`.                                            |

```json
{
  "id": "village-props-20261031140200",
  "choice": "raise",
  "key": "garden_tools",
  "triangles": 3000
}
```

### list_animations

`read only`

Lists the clips in the [Animations](/docs/views/animations/) library, kept once outside every project and skeleton only: id, name, pack, tags, source rig (`mixamo`, `tripo` or `custom`), bone count, frames, rate, seconds, root motion, loop, size, revision and the file it came from.

| Parameter | Type   | Required | Notes                                           |
| --------- | ------ | -------- | ----------------------------------------------- |
| `query`   | string | no       | Words to find in the clip names, packs, tags.   |
| `pack`    | string | no       | Only clips in this pack.                        |
| `tag`     | string | no       | Only clips with this tag.                       |
| `rig`     | string | no       | Only clips made on `mixamo`, `tripo`, `custom`. |

### import_animations

`changes the library`

Imports Mixamo FBX files or pack zips that are inside the workspace, as Import clips does. Blender reads them headless: each FBX is one clip, the skin is dropped and only the skeleton and keys are kept, and the rig is read from the bone names. A file with no keys is left out. A clip whose keys the library has is skipped unless `keepDuplicates`; new keys for a clip of the same name and pack replace it as its next revision. A zip's clips go to a pack named after it, single files to Singles. A path outside the workspace is refused: copy the files in first. It spends nothing.

| Parameter        | Type     | Required | Notes                                                          |
| ---------------- | -------- | -------- | -------------------------------------------------------------- |
| `paths`          | string[] | yes      | Absolute, or relative to the workspace root.                   |
| `singles`        | string   | no       | The pack single files go to. Singles by default.               |
| `keepDuplicates` | boolean  | no       | Add clips the library has already, as clips of their own.      |
| `keepFiles`      | boolean  | no       | Keep each FBX next to its clip. Off by default.                |
| `dryRun`         | boolean  | no       | Return what would be added, left out and skipped; add nothing. |

```json
{ "paths": ["drop/Pro Magic Pack.zip", "drop/Standing Dodge Left.fbx"] }
```

### save_model_clips

`changes the library`

Saves a rigged model's clips to the animation library as a pack, as Save clips to library does: a project asset's newest version (`project` and `asset`), or a model waiting in the Inbox (`inbox`, its file name), such as a Tripo job's animations. Only the skeleton and the clips' keys go in. Clips that came from the library are never saved back and keys the library has are skipped. Runs Blender headless and spends nothing.

| Parameter | Type     | Required | Notes                                                     |
| --------- | -------- | -------- | --------------------------------------------------------- |
| `project` | string   | no       | The project id, with `asset`.                             |
| `asset`   | string   | no       | The asset id.                                             |
| `inbox`   | string   | no       | An Inbox file name, in place of `project` and `asset`.    |
| `clips`   | string[] | no       | Clip names. Every clip not from the library by default.   |
| `pack`    | string   | no       | The pack. `<model> moves` by default.                     |
| `tags`    | string[] | no       | Tags for every clip. Read from the clip names by default. |
| `dryRun`  | boolean  | no       | Return the model's rig and clips; save nothing.           |

```json
{ "project": "frostveil", "asset": "elsheria", "pack": "Elsheria moves" }
```

### use_animations

`changes the project`

Puts library clips on a rigged model, as Use on model does. The clip's skeleton is mapped onto the model's rig: the map saved for the model first, then the bone names (Mixamo and Tripo names included); `map` sets bones by hand, and a bone left unmapped stays at rest. Each clip gets its name, frames, loop, root motion or in place, and speed on this model only; nothing is written back to the library. Blender bakes the clips headless onto a copy of the newest version and reviews each one: `good`, `look` (a planted foot slides more than 1.5 cm) or `failed` (keys in the wrong units, nothing mapped). Failed clips are left out and the rest become the model's next version through the usual checks; the older version stays. A clip the model baked before is replaced by the new bake. `pinFeet` on a clip holds its planted feet still (a second headless run solves the legs back with IK) and the review gives the slide before as `unpinnedCm` and after as `slideCm`. Spends nothing. Open in Blender has no tool: it is a click in the app only.

| Parameter | Type     | Required | Notes                                                                                                                                      |
| --------- | -------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `project` | string   | yes      | The project id.                                                                                                                            |
| `asset`   | string   | yes      | The asset id of a rigged model.                                                                                                            |
| `clips`   | object[] | yes      | `{ clip, name?, start?, end?, loops?, rootMotion?, speed?, pinFeet? }`: the library clip id and its settings; defaults come from the clip. |
| `map`     | object[] | no       | `{ bone, target? }`: a humanoid bone (`Neck`, `LeftArm`) and the model bone it drives; no `target` keeps it at rest.                       |
| `saveMap` | boolean  | no       | Keep the map for the model's next retarget. On by default.                                                                                 |
| `dryRun`  | boolean  | no       | Bake and review only; add no version.                                                                                                      |

```json
{
  "project": "frostveil",
  "asset": "elsheria",
  "clips": [{ "clip": "standing-dodge-left", "start": 4, "end": 33, "speed": 1.15 }]
}
```

### get_retarget_review

`read only`

A model's last retarget review: each library clip baked onto it with its status, a one line note, its planted foot slide (`slideCm`, and `unpinnedCm` before Pin the feet when it was pinned), the bones driven and left at rest, and the version it made, with every library clip baked into the model and the revision it was baked at.

| Parameter | Type   | Required | Notes           |
| --------- | ------ | -------- | --------------- |
| `project` | string | yes      | The project id. |
| `asset`   | string | yes      | The asset id.   |
