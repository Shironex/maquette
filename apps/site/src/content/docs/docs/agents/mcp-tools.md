---
title: MCP tools
description: Every tool the built in MCP server offers, what it does, and whether it spends credits or needs approval in the app.
sidebar:
  label: MCP tools
---

The server offers 156 tools. Set it up first on [Connect an agent](/docs/agents/connect/). Start with `get_started`, then `create_project` when there is no project to work in. Tools that work on a project take its id as `project`, as `list_projects` returns it.

## Tool groups

Every tool belongs to one group, and its description starts with that group in brackets. Every agent session pays context for each tool it is offered, so Settings > Agents lists the groups with a switch each: turn off the ones you never use for a shorter tool list. All are on by default.

| Group          | Tools | What it covers                                                                           |
| -------------- | ----- | ---------------------------------------------------------------------------------------- |
| `[start]`      | 2     | `get_started` and `create_project`. Always on.                                           |
| `[project]`    | 9     | Projects, assets, the scene layout, copying assets and the asset trash. Always on.       |
| `[lab]`        | 14    | Concept images and their jobs, the style board, picks, image backends and Codex usage.   |
| `[refs]`       | 5     | An asset's reference views: tagging, drawing missing ones, cleaning backgrounds.         |
| `[plan]`       | 14    | Planning canvases and their nodes, the change log and a website's camera plan.           |
| `[send]`       | 14    | Tripo, Meshy and by hand sends, redos, splits, credits, handoffs and downloads.          |
| `[inbox]`      | 7     | Models waiting to be accepted: checks, accept, discard and restore.                      |
| `[prepare]`    | 14    | Real size, fix-ups, parts, materials, assembling props and comparing versions.           |
| `[built]`      | 4     | Models a Blender script in the project makes, with no credits.                           |
| `[pipeline]`   | 17    | Runs, presets, bake settings, starter scripts, shipping and the site's dev server.       |
| `[game]`       | 5     | A game project's repo, format, budgets, imports and ship plan.                           |
| `[library]`    | 12    | Models kept outside any project, copied in and out.                                      |
| `[animation]`  | 24    | The clip library, baking clips onto a rigged model, animation recipes and their library. |
| `[export]`     | 8     | Export packs for other engines and render styles.                                        |
| `[avatar-rig]` | 7     | VRM avatars and the rig doctor.                                                          |

What turning a group off does:

- `tools/list` leaves its tools out. The start and project groups cannot be turned off, so an agent can always find its way.
- Connected agents get `notifications/tools/list_changed` when a save changes the groups, and the next list they ask for follows the setting. A client that ignores the notice keeps its old list until it reconnects.
- A call to a tool of an off group (an agent with an older list) is refused and nothing runs: the error has `code` `group-off`, names the group, says the person turned it off in Settings > Agents and carries `personOnly`.
- `get_started` adds `toolGroupsOff`, one line naming the groups that are off with their tool counts, and leaves it out when all are on.
- The setting is saved in the workspace's `maquette.json` as `agents.rules.toolGroupsOff`. A workspace from before the groups that had Offer avatar and rig tools off opens with the avatar and rig group off.

A tool that fails answers JSON: `error` (the message), `code` (a stable word such as `not-found`, `invalid`, `taken`, `refused` or `preview-refused`), and where they help `fix`, `tryTool` and `personOnly` (true when only a person can fix it, such as a missing key, the Codex sign in or Blender). Empty answers carry a `next` list, and an answer with status `waiting for approval` carries `pollEverySeconds` and `whereToApprove`.

How to read the tables: **Spends** says whether a call costs anything. **Approval** says whether a person has to answer in the Maquette app before it goes on.

Jump to a group: [Read only](#read-only), [Changes the project, spends nothing](#changes-the-project-spends-nothing), [Spends credits, Codex usage or dollars, or needs approval](#spends-credits-codex-usage-or-dollars-or-needs-approval), [The main tools](#the-main-tools).

## Read only

These look at the workspace and change nothing.

| Tool                                            | What it returns                                                                                                                                                                                                                                                                                                                                                                                                    | Spends  | Approval |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------- | -------- |
| [`get_started`](#get_started)                   | Where to start: a short answer with counts, the toolchain summary, image backend, projects with target and stage, blocking gates and a `next` list; `detail: true` gives everything                                                                                                                                                                                                                                |
| [`list_projects`](#list_projects)               | Every project: template, target (website or game), site or game repo, asset count, what runs in it and which is open in the app                                                                                                                                                                                                                                                                                    | Nothing | No       |
| `get_project`                                   | One project's manifest: name, template, site repo, source folder and its Board assets with every version                                                                                                                                                                                                                                                                                                           | Nothing | No       |
| `list_assets`                                   | A project's Board assets: kind, stage, status, version count, the latest version's file and stats, and its refs with their views (`refsTagged` false while they are read from the refs' order). Takes an optional `stage` filter                                                                                                                                                                                   | Nothing | No       |
| [`get_pipeline`](#get_pipeline)                 | Steps, validation issues, compression presets, model sizes, the run in progress and the latest runs                                                                                                                                                                                                                                                                                                                | Nothing | No       |
| [`get_run`](#get_run)                           | One run's status and the end of its log                                                                                                                                                                                                                                                                                                                                                                            | Nothing | No       |
| `list_inbox`                                    | The models waiting in the inbox, with kind, size, triangle count, worst check, the version each would become and the newer version that replaced it, plus the discarded models                                                                                                                                                                                                                                     | Nothing | No       |
| `get_inbox_checks`                              | The checks for one inbox model (triangles, textures, rig, VRM readiness, scale, materials) and its analysis report (with `vrm`: the bone map, rest pose measures and findings), read against its send as `list_inbox` does                                                                                                                                                                                         | Nothing | No       |
| `get_concepts`                                  | A direction's prompt, refs and images with stars and picks, plus queued, running, waiting and failed jobs                                                                                                                                                                                                                                                                                                          | Nothing | No       |
| `get_style_board`                               | A project's style board: the style line, the palette, up to 4 images with the concept or ref each came from, and its revision                                                                                                                                                                                                                                                                                      | Nothing | No       |
| `get_codex_usage`                               | How much of Codex's five hour and weekly windows is used, when each resets, whether the usage limit holds and until when. `known` is false with a reason when this Codex CLI cannot say. Lab jobs waiting for the limit move to the reset time it names                                                                                                                                                            | Nothing | No       |
| [`get_image_backend`](#set_image_backend)       | The Lab's backend per kind of job (`codex` or `openai-images`), the Image API's quality and monthly cap in dollars, this month's spend and what queued jobs will likely cost, whether an OpenAI key is saved, and the estimate per image                                                                                                                                                                           | Nothing | No       |
| [`get_ref_views`](#set_ref_views)               | An asset's refs with the view each shows, whether they were tagged, the candidates `make_other_views` and `clean_ref_background` made and their jobs still running                                                                                                                                                                                                                                                 | Nothing | No       |
| `get_provider_job`                              | A send to 3D: status, phase, progress, credits spent, message and `decision` (who answered its approval). With `project` instead of `id`, every send in the project                                                                                                                                                                                                                                                | Nothing | No       |
| `cancel_provider_jobs`                          | Stops sends by id (`ids`, 1 to 50). A by hand send stops waiting, a send not yet started is cancelled, and each id answers on its own. Sends waiting for approval are refused: you deny those in the app                                                                                                                                                                                                           | Nothing | No       |
| `cancel_concept_jobs`                           | Cancels Lab jobs as the Lab queue's Cancel does (`{ project, jobs? }`, every unfinished job without `jobs`): queued, running, waiting out a limit or waiting for approval. Each job answers with its state or why it was not cancelled                                                                                                                                                                             | Nothing | No       |
| `prioritize_concept_jobs`                       | Run next: moves a project's queued Lab jobs (or the named ones) to the front of the line, ahead of other projects' jobs. Running, waiting and finished jobs are listed as skipped                                                                                                                                                                                                                                  | Nothing | No       |
| [`get_credits`](#get_credits)                   | Each provider's balance, totals for 7 and 30 days and the latest paid sends, and the month's OpenAI image dollars against the cap                                                                                                                                                                                                                                                                                  | Nothing | No       |
| [`list_handoffs`](#list_handoffs)               | The By hand sends waiting for a model, the export file name each waits for, and the Downloads watcher: downloads that need a look (`needsLook`) and ones it could not place (`unmatched`)                                                                                                                                                                                                                          | Nothing | No       |
| `list_removed_assets`                           | A project's removed assets waiting in the workspace trash, newest first                                                                                                                                                                                                                                                                                                                                            | Nothing | No       |
| `get_real_size`                                 | An asset's real size and fix-ups, what its model measures, and the defaults new fix-ups start from                                                                                                                                                                                                                                                                                                                 | Nothing | No       |
| `list_fixups`                                   | An asset's fix-ups in the order they apply, with the last preview's numbers                                                                                                                                                                                                                                                                                                                                        | Nothing | No       |
| [`get_material_tweaks`](#get_material_tweaks)   | An asset's material names, tweaks and per map group codecs, with raw and shipped size                                                                                                                                                                                                                                                                                                                              | Nothing | No       |
| [`get_avatar`](#get_avatar)                     | A character asset's VRM avatar: its readiness (bone map, rest pose measures, findings), the meta, bone overrides and material mode, the last normalised rest pose with its sheets and state, the parts that could swing and the last spring chains, and the last export                                                                                                                                            | Nothing | No       |
| `get_bake_settings`                             | Lightmap size, samples, denoiser, whether `bake_light.py` can denoise, and the assets that skip the bake                                                                                                                                                                                                                                                                                                           | Nothing | No       |
| `get_starters`                                  | Each starter script in `tools/`, and the built asset helpers in `built/`, as current, outdated, edited or missing                                                                                                                                                                                                                                                                                                  | Nothing | No       |
| `get_layout`                                    | Where each model stands, the camera stops, the desk, the first view, object states, explode tracks, paths and clips kept off the site                                                                                                                                                                                                                                                                              | Nothing | No       |
| `get_motion`                                    | Every model's rig, clips and parts, and what the clips weigh inside the scene budget                                                                                                                                                                                                                                                                                                                               | Nothing | No       |
| [`get_parts`](#set_parts)                       | An asset's parts as its newest model ships them: the name in the model, the name it ships as, triangles, materials, hidden and joined parts. Runs its split headless on a copy first when it has not run                                                                                                                                                                                                           | Nothing | No       |
| `compare_versions`                              | Two versions of an asset (or one before and after its fix-ups) part by part: the parts Motion can move, draw calls, triangles, textures, transfer size and Inbox checks                                                                                                                                                                                                                                            | Nothing | No       |
| [`get_model_facts`](#get_model_facts)           | The measured facts of an asset version: counts, the file's box, real size, the box at real size, facing with how it was decided, the layout entry and the ref images' paths                                                                                                                                                                                                                                        |
| `list_built_assets`                             | Each built asset's script, helpers, state and last build                                                                                                                                                                                                                                                                                                                                                           | Nothing | No       |
| `get_page_preview`                              | The site dev server's state, address, command, who started it and its last log lines                                                                                                                                                                                                                                                                                                                               | Nothing | No       |
| `get_game`                                      | A game project's settings, each model against its folder's budget and the game templates (add `previews` for what each would change here)                                                                                                                                                                                                                                                                          | Nothing | No       |
| `scan_game_models`                              | Every model in a game project's model folder with its numbers, budget check and the asset it was imported as                                                                                                                                                                                                                                                                                                       | Nothing | No       |
| `get_ship_plan`                                 | What Ship would write: for a game, new, replaced (marked when an imported original), unchanged and hand made files and the uncommitted file count (not read for a repo an agent named or set), and the old files a moved asset leaves (removed); `repo` plans against another checkout or worktree; for a website, each file written or held back and why, with sizes before and after, and the site's other files | Nothing | No       |
| [`get_ship_settings`](#get_ship_settings)       | A website's Ship style, texture format and quality, each asset's choice and file name, the site folders, the plan and the size report                                                                                                                                                                                                                                                                              | Nothing | No       |
| [`get_size_report`](#get_size_report)           | Each model's source bytes and triangles against what ships, with totals, as rows, CSV and Markdown                                                                                                                                                                                                                                                                                                                 | Nothing | No       |
| [`list_library`](#list_library)                 | The library's models, with their versions, tags and the projects that use them. Takes an optional `query`, `tag`, `project`, `collection`, `limit`, `cursor` and `full`                                                                                                                                                                                                                                            | Nothing | No       |
| [`find_look_alikes`](#find_look_alikes)         | Library entries that may be one model twice (same look or same name), the entries with no readable picture, the recent merges and the settings                                                                                                                                                                                                                                                                     | Nothing | No       |
| [`get_pack`](#get_pack)                         | One export pack: its state, each model's check with triangles before and after, and what needs you                                                                                                                                                                                                                                                                                                                 | Nothing | No       |
| [`get_render`](#get_render)                     | One render: its state, each model's files and what stopped them                                                                                                                                                                                                                                                                                                                                                    | Nothing | No       |
| [`list_renders`](#list_renders)                 | The newest renders                                                                                                                                                                                                                                                                                                                                                                                                 | Nothing | No       |
| [`list_packs`](#list_packs)                     | The newest export packs and the pack presets                                                                                                                                                                                                                                                                                                                                                                       | Nothing | No       |
| [`get_rig_doctor`](#get_rig_doctor)             | A rig doctor case: its sheets, defects, checks, progress and why it stopped                                                                                                                                                                                                                                                                                                                                        | Nothing | No       |
| [`list_rig_doctors`](#list_rig_doctors)         | The rig doctor's cases, newest first                                                                                                                                                                                                                                                                                                                                                                               | Nothing | No       |
| [`list_animations`](#list_animations)           | The animation library's clips: pack, tags, source rig, bone count, frames, rate, seconds, root motion, loop and size. Takes an optional `query`, `pack`, `tag` and `rig`                                                                                                                                                                                                                                           | Nothing | No       |
| [`list_recipes`](#list_recipes)                 | A project's animation recipes: id, name, base clip and frames, whether the base clip is Mixamo's, and the change and blank counts with how many blanks are open                                                                                                                                                                                                                                                    | Nothing | No       |
| [`get_recipe`](#get_recipe)                     | One animation recipe: its base clip as matched in this computer's library, its blanks and its changes in order                                                                                                                                                                                                                                                                                                     | Nothing | No       |
| [`get_recipe_bake`](#get_recipe_bake)           | A recipe's last bake: its state, progress, why it stopped, the staged result and its review against the limits                                                                                                                                                                                                                                                                                                     | Nothing | No       |
| [`get_recipe_pose`](#get_recipe_pose)           | The turns a recipe's pose keys make at one frame, blended as the preview blends them, against the humanoid joint limits                                                                                                                                                                                                                                                                                            | Nothing | No       |
| [`list_library_recipes`](#list_library_recipes) | The recipe library's entries: each revision with where it came from, the newest base clip and counts                                                                                                                                                                                                                                                                                                               | Nothing | No       |
| [`get_library_recipe`](#get_library_recipe)     | One library recipe's revision in full, its base clip as matched here, and every revision                                                                                                                                                                                                                                                                                                                           | Nothing | No       |
| `list_grips`                                    | The grip library: each grip's weapon, hands, which hand holds, bone count and every revision with where it was saved from                                                                                                                                                                                                                                                                                          |
| `get_grip`                                      | One library grip's revision with its pose in degrees per hand, finger and arm bone, and every revision                                                                                                                                                                                                                                                                                                             |
| [`list_canvases`](#list_canvases)               | A project's planning canvases: id, name, preset, node and link counts and when each last changed                                                                                                                                                                                                                                                                                                                   |
| [`get_canvas`](#get_canvas)                     | A planning canvas: its nodes and the links between them, each node with who added it                                                                                                                                                                                                                                                                                                                               |
| [`get_changes`](#get_changes)                   | A planning canvas's change log, newest first, with the change ids `undo_run` takes                                                                                                                                                                                                                                                                                                                                 |
| [`get_camera_plan`](#get_camera_plan)           | A website project's camera plan: the main line Apply would write, the agents' proposals and whether layout.json changed since                                                                                                                                                                                                                                                                                      |
| [`get_apply_preview`](#get_apply_preview)       | What Apply would write to layout.json from the camera plan: every added, moved, removed and changed stop                                                                                                                                                                                                                                                                                                           |

`preview_fixups` is read only too: it runs an asset's saved fix-ups headless on a copy and returns the numbers before and after each. It spends nothing.

## Changes the project, spends nothing

| Tool                                                  | What it does                                                                                                                                                                                                                                                           | Spends  | Approval |
| ----------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- | -------- |
| [`set_compression_presets`](#set_compression_presets) | Changes the per model texture size, data map size, codec and meshopt level                                                                                                                                                                                             | Nothing | No       |
| [`set_material_tweaks`](#set_material_tweaks)         | Changes an asset's tint, roughness, metalness, emissive strength and map codecs                                                                                                                                                                                        | Nothing | No       |
| [`set_avatar`](#set_avatar)                           | Changes a character asset's VRM meta, bone map overrides, material mode and confirmed springs                                                                                                                                                                          | Nothing | No       |
| [`create_project`](#create_project)                   | Makes a website or game project, as New project does; refuses a taken name                                                                                                                                                                                             |
| [`set_ship_settings`](#set_ship_settings)             | Changes a website's site repo, Ship style, texture format, quality, per asset choice and file name, and the site folders                                                                                                                                               | Nothing | No       |
| `set_bake_settings`                                   | Changes the lightmap size (256 to 4096), samples (1 to 4096) and denoise                                                                                                                                                                                               | Nothing | No       |
| `set_skip_bake`                                       | Sets one or more assets to skip the bake, or join it again                                                                                                                                                                                                             | Nothing | No       |
| `set_pad_edges`                                       | Pads an asset's texture edges before the textures shrink, or stops                                                                                                                                                                                                     | Nothing | No       |
| [`set_image_backend`](#set_image_backend)             | Sets the Lab's backend per kind of job and the Image API's quality. The monthly cap and the key are only ever set in the app                                                                                                                                           | Nothing | No       |
| `set_real_size`                                       | Sets or clears the real size a model is scaled to                                                                                                                                                                                                                      | Nothing | No       |
| `add_fixups`                                          | Adds fix-ups to an asset, Split into parts (`split-parts`) and Rotate (`rotate` with degrees `x`, `y`, `z`) among them; one of a kind already there is replaced                                                                                                        | Nothing | No       |
| [`set_parts`](#set_parts)                             | Renames, joins, unjoins, hides or shows an asset's parts                                                                                                                                                                                                               | Nothing | No       |
| [`assemble_prop`](#assemble_prop)                     | Joins part models into one prop, a node per part under one root, as the next version of an asset                                                                                                                                                                       | Nothing | No       |
| [`bake_back`](#bake_back)                             | Bakes the texture back onto a split that lost it, headless, as the asset's next version                                                                                                                                                                                | Nothing | No       |
| `remove_fixups`                                       | Removes fix-ups by kind, or all of them                                                                                                                                                                                                                                | Nothing | No       |
| `update_starters`                                     | Brings outdated or missing starter scripts up to date, as the Update banner does. Built assets that list an updated helper file show as stale                                                                                                                          | Nothing | No       |
| [`set_layout`](#set_layout)                           | Changes placements, camera stops, desk, object states, explode tracks, paths and clips                                                                                                                                                                                 | Nothing | No       |
| [`pick_concepts`](#pick_concepts)                     | Puts Lab concepts on the Board at Refs                                                                                                                                                                                                                                 | Nothing | No       |
| `star_concepts`                                       | Stars or unstars Lab concepts                                                                                                                                                                                                                                          | Nothing | No       |
| [`set_style_board`](#set_style_board)                 | Changes a project's style board: the line, the palette and the images, copied into `style/`                                                                                                                                                                            | Nothing | No       |
| [`set_ref_views`](#set_ref_views)                     | Sets the view of every ref of an asset: front, left, back, right or other                                                                                                                                                                                              | Nothing | No       |
| `use_ref_candidate`                                   | Takes a drawn view into its slot (the ref it replaces moves to `refs/originals`), or a clean copy into the place of the ref it cleans (the original moves there too)                                                                                                   | Nothing | No       |
| [`accept_inbox_model`](#accept_inbox_model)           | Moves an inbox model into a project as a new asset version                                                                                                                                                                                                             | Nothing | No       |
| [`discard_inbox_model`](#discard_inbox_model)         | Moves an inbox model aside into `inbox/.discarded/` instead of deleting it                                                                                                                                                                                             | Nothing | No       |
| [`accept_inbox_models`](#accept_inbox_models)         | Accepts several inbox models in one call, each answering on its own                                                                                                                                                                                                    | Nothing | No       |
| [`restore_inbox_model`](#restore_inbox_model)         | Puts a discarded inbox model back into the inbox, aimed at the asset it was going to                                                                                                                                                                                   | Nothing | No       |
| [`attach_download`](#attach_download)                 | Sends a download from `list_handoffs` to the inbox for an asset                                                                                                                                                                                                        | Nothing | No       |
| [`add_model_file`](#add_model_file)                   | Adds a model file you fixed yourself as an asset's next version, through the inbox                                                                                                                                                                                     | Nothing | No       |
| `dismiss_download`                                    | Leaves a download from `list_handoffs` alone and never asks about it again                                                                                                                                                                                             | Nothing | No       |
| `copy_asset`                                          | Copies an asset with its refs and versions to another project                                                                                                                                                                                                          | Nothing | No       |
| `remove_assets`                                       | Takes up to 50 assets off the Board into the workspace trash, stopping their waiting sends                                                                                                                                                                             | Nothing | No       |
| `restore_asset`                                       | Puts a removed asset back from the workspace trash with its files, place and settings                                                                                                                                                                                  | Nothing | No       |
| `import_game_models`                                  | Imports models from the game repo, each as an asset whose version 1 is the repo file, and asks for their previews                                                                                                                                                      | Nothing | No       |
| `set_game_settings`                                   | Changes a game's format, model folder, export (dropping normal and metal maps too), bake, folder budgets, each asset's own folder (`assetFolders`) and the game repo (`repo`, a checkout or worktree), or applies a game template (`template`, an id `get_game` lists) | Nothing | No       |
| `add_built_asset`                                     | Adds a model a [Blender script in the project](/docs/guides/built-assets/) makes, and builds it headless                                                                                                                                                               | Nothing | No       |
| `rebuild_built_asset`                                 | Builds one built asset again, or every new or stale one                                                                                                                                                                                                                | Nothing | No       |
| `replace_with_built_asset`                            | Puts a built asset in the place of another model, which keeps its id, placement and ship name; `dryRun` reports only, `undo` brings the old model back (refused while `notBuilt` lists a version)                                                                      | Nothing | No       |
| `render_thumbnail`                                    | Renders a studio thumbnail of a model with headless Blender and returns the PNG's path                                                                                                                                                                                 | Nothing | No       |
| [`render_views`](#render_views)                       | Renders an asset version from the standard views in the page light, beside its refs, or two versions with the same cameras                                                                                                                                             | Nothing |
| `start_page_preview`                                  | Runs the site repo's dev command: starts the server the Page view shows, or reuses the running one. The first start of a repo an agent set waits for your approval                                                                                                     | Nothing | No       |
| `stop_page_preview`                                   | Stops that server, whoever started it                                                                                                                                                                                                                                  | Nothing | No       |
| `cancel_run`                                          | Cancels a queued or running run. A run waiting for approval is cancelled too                                                                                                                                                                                           | Nothing | No       |
| [`add_to_library`](#add_to_library)                   | Copies an asset's newest model into the library                                                                                                                                                                                                                        | Nothing | No       |
| [`add_many_to_library`](#add_many_to_library)         | Copies up to 200 assets' newest models into the library in one call, each answering on its own                                                                                                                                                                         | Nothing | No       |
| [`use_from_library`](#use_from_library)               | Copies a library model into a project as a new asset                                                                                                                                                                                                                   | Nothing | No       |
| [`update_from_library`](#update_from_library)         | Adds a newer library version to an asset as its next version, or returns the plan                                                                                                                                                                                      | Nothing | No       |
| [`tag_library_entries`](#tag_library_entries)         | Puts tags on and takes tags off several library entries at once                                                                                                                                                                                                        | Nothing | No       |
| [`rename_library_entry`](#rename_library_entry)       | Renames a library entry; its id stays                                                                                                                                                                                                                                  | Nothing | No       |
| [`set_library_collection`](#set_library_collection)   | Creates, fills, empties, renames or deletes a library collection                                                                                                                                                                                                       | Nothing | No       |
| [`remove_from_library`](#remove_from_library)         | Moves library entries to `library/.removed`; the copies in projects stay                                                                                                                                                                                               | Nothing | No       |
| [`restore_to_library`](#restore_to_library)           | Puts removed library entries back, undoing a merge for a merged one                                                                                                                                                                                                    | Nothing | No       |
| [`merge_library_entries`](#merge_library_entries)     | Merges one library entry into another and points project assets along                                                                                                                                                                                                  | Nothing | No       |
| [`export_pack`](#export_pack)                         | Writes library models as one pack (OBJ or GLB, README, manifest, contact sheet, zip) inside the workspace, each checked in headless Blender                                                                                                                            |
| [`start_rig_doctor`](#start_rig_doctor)               | Opens the rig doctor's case of a rigged model and reads it, with no Blender                                                                                                                                                                                            |
| [`run_rig_doctor`](#run_rig_doctor)                   | Starts a case's pose test, fix or second round in headless Blender on a copy                                                                                                                                                                                           |
| [`decide_pack`](#decide_pack)                         | Raises a pack model's budget, keeps a warning, exports it again, zips, stops or goes on                                                                                                                                                                                |
| [`render_styles`](#render_styles)                     | Renders models in headless Blender: blueprint, toon, clay, shaded, turntable, contact sheet                                                                                                                                                                            |
| [`decide_render`](#decide_render)                     | Stops a render or goes on with a stopped one                                                                                                                                                                                                                           |
| [`import_animations`](#import_animations)             | Imports Mixamo FBX files or pack zips into the animation library, skeleton only                                                                                                                                                                                        | Nothing | No       |
| [`save_model_clips`](#save_model_clips)               | Saves a rigged model's clips to the animation library as a pack                                                                                                                                                                                                        | Nothing | No       |
| [`use_animations`](#use_animations)                   | Bakes library clips onto a rigged model as its next version, after a review                                                                                                                                                                                            | Nothing | No       |
| [`create_recipe`](#create_recipe)                     | Makes an animation recipe on a library clip, as the Motion view's Recipes tab does; bakes nothing                                                                                                                                                                      | Nothing | No       |
| [`update_recipe`](#update_recipe)                     | Changes a recipe's name, changes or blanks, checked whole before anything is written; bakes nothing                                                                                                                                                                    | Nothing | No       |
| [`set_recipe_keys`](#set_recipe_keys)                 | Adds, moves or removes pose keys on one pose key change by bone and frame, checked like update_recipe; bakes nothing                                                                                                                                                   | Nothing | No       |
| [`bake_recipe`](#bake_recipe)                         | Bakes a recipe onto a rigged model in headless Blender, staged for a person to review; answers at once                                                                                                                                                                 | Nothing | No       |
| [`use_recipe`](#use_recipe)                           | Copies a library recipe's revision into a project as a new recipe that remembers it                                                                                                                                                                                    | Nothing | No       |
| [`import_recipe`](#import_recipe)                     | Imports a `.recipe.json` inside the workspace into a project, checked whole first                                                                                                                                                                                      | Nothing | No       |
| [`export_recipe`](#export_recipe)                     | Writes a recipe to a new `.recipe.json` under `exports/recipes`, with no library link and no keys                                                                                                                                                                      | Nothing | No       |
| `apply_grip`                                          | Lays a library grip on a recipe as a grip change holding its own copy of the pose and a link back, as Add a grip does; bakes nothing                                                                                                                                   |
| [`get_retarget_review`](#get_retarget_review)         | A model's last retarget review and the library clips baked into it                                                                                                                                                                                                     | Nothing | No       |
| [`create_canvas`](#create_canvas)                     | Makes a planning canvas in a project, blank or from the Story or Game mechanics preset                                                                                                                                                                                 |
| [`add_nodes`](#add_nodes)                             | Adds nodes and links to a planning canvas, placed beside a node, clear of the others                                                                                                                                                                                   |
| [`update_nodes`](#update_nodes)                       | Edits or moves nodes on a planning canvas; only the fields given change                                                                                                                                                                                                |
| [`remove_nodes`](#remove_nodes)                       | Removes nodes and links the agent added; a person's work is refused                                                                                                                                                                                                    |
| [`layout_canvas`](#layout_canvas)                     | Moves the agent's own overlapping nodes to free spots                                                                                                                                                                                                                  |
| [`undo_run`](#undo_run)                               | Undoes the agent's own changes on a planning canvas, a whole run or some changes                                                                                                                                                                                       |
| [`propose_removal`](#propose_removal)                 | Asks the person to remove nodes the agent may not remove; only the person answers                                                                                                                                                                                      |
| [`start_from_node`](#start_from_node)                 | Starts a new planning canvas from a node and the nodes linked to it; the source stays                                                                                                                                                                                  |
| [`propose_camera_plan`](#propose_camera_plan)         | Proposes a camera tour as the agent's own line beside the person's; never touches layout.json                                                                                                                                                                          |

## Spends credits, Codex usage or dollars, or needs approval

| Tool                                          | What it does                                                                                                                                                                                                                          | Spends                                                          | Approval                                                                                                   |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| [`run_pipeline`](#run_pipeline)               | Runs a project's pipeline, optionally from a step or build only                                                                                                                                                                       | Nothing                                                         | At the Ship step, when Ask before shipping is on, and always in a game or into a repo an agent chose       |
| [`send_to_3d`](#send_to_3d)                   | Sends an asset's refs to Tripo or Meshy, or writes a By hand folder                                                                                                                                                                   | Credits for Tripo and Meshy. By hand spends nothing             | Always, for Tripo and Meshy                                                                                |
| `send_batch_to_3d`                            | Sends up to 24 assets at once with the same provider, with each asset's weak ref warnings and a modelWarning when it has a model                                                                                                      | Credits for Tripo and Meshy                                     | One approval for the whole batch                                                                           |
| `redo_inbox_model`                            | Sends an inbox model's asset again, prefilled from its send                                                                                                                                                                           | Credits for Tripo and Meshy                                     | Always, for Tripo and Meshy                                                                                |
| `redo_inbox_models`                           | Redoes several inbox models from one project as one batch                                                                                                                                                                             | Credits for Tripo and Meshy                                     | One approval for the whole batch                                                                           |
| [`split_with_provider`](#split_with_provider) | Splits an asset's model into parts on Tripo (segmentation) or Meshy (Auto Split)                                                                                                                                                      | Credits for Tripo and Meshy                                     | Always                                                                                                     |
| [`build_avatar`](#build_avatar)               | Builds a character asset's VRM 1.0 avatar into the project's `exports/<asset>.vrm`, freeing cloth and hair from the arms and normalising its rest pose headless first when it needs it                                                | Nothing                                                         | When the weights or the rest pose change: a person approves the before and after sheets in the Avatar view |
| [`add_recipe_bake`](#add_recipe_bake)         | Asks a person to add a recipe's finished bake as its model's next version; they review it in the Motion view and add it there                                                                                                         | Nothing                                                         | Always: only the Add in the review adds it                                                                 |
| [`propose_recipe_save`](#propose_recipe_save) | Asks a person to save a project's recipe to the recipe library as a new revision or a new entry                                                                                                                                       | Nothing                                                         | Always: only the save in the sheet writes                                                                  |
| `propose_grip_save`                           | Asks a person to save the hand pose a recipe's pose keys hold at one frame to the grip library as a new grip or the next revision                                                                                                     |
| `start_concepts`                              | Starts concept image jobs in the Lab, with the project style unless `useStyle` is false. `backend` picks Codex or OpenAI Images for this request                                                                                      | Codex usage, or dollars on OpenAI Images within the monthly cap | OpenAI Images only, when an agent asks                                                                     |
| `make_object_refs`                            | Makes one isolated image per object of a concept, with the project style unless `useStyle` is false. `backend` picks Codex or OpenAI Images for this request                                                                          | Codex usage, or dollars on OpenAI Images within the monthly cap | OpenAI Images only, when an agent asks                                                                     |
| `make_other_views`                            | Draws an asset's missing views (or the named ones) from its front ref, 1 to 4 candidates each, with the project style unless `useStyle` is false and `notes` in every prompt. `backend` picks Codex or OpenAI Images for this request | Codex usage, or dollars on OpenAI Images within the monthly cap | OpenAI Images only, when an agent asks                                                                     |
| `clean_ref_background`                        | Makes a copy of one ref with the object kept and the clutter cleared, on studio grey or transparent. `backend` picks Codex or OpenAI Images for this request                                                                          | Codex usage, or dollars on OpenAI Images within the monthly cap | OpenAI Images only, when an agent asks                                                                     |
| `retry_concept_jobs`                          | Runs failed or cancelled Lab jobs again, including jobs the app closed on. `include: "held"` or `"all"` runs the jobs waiting out a rate or usage limit now too                                                                       | Codex usage, or dollars on OpenAI Images within the monthly cap | OpenAI Images only, when an agent asks                                                                     |
| `retry_usage_limited_jobs`                    | Runs the Lab jobs waiting for the Codex usage limit now, but only when a fresh read says the limit has lifted; otherwise says until when it holds                                                                                     | Codex usage                                                     | No                                                                                                         |

## The main tools

### get_started

Works with no workspace open. The answer is short by default: counts, the `toolchain` as working, of and problems, the image backend, `projects` with `id`, `name`, `target`, `stage` (`empty`, or the earliest stage an asset is at) and assets, the gates that block, `next` and `openProject`. Pass `detail: true` for everything: `toolchain` lists each tool with `status` (found, missing or broken), `version`, `hint` and `warning`, `keys` has `tripo`, `meshy` and `openai` as true or false (never a value), `projects` adds `stages` (assets per stage), and `personGates` lists every gate only a person can pass, Approve credits included, with where. `next` lists what to do now: each step has `why`, and either `tool` or `person: true`.

| Argument | What it does                         |
| -------- | ------------------------------------ |
| `detail` | Optional. `true` for the full answer |

### create_project

| Argument   | What it does                                                                                                                                                                                     |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `name`     | The name. One another project has, or that makes the same id, is refused with code `taken`                                                                                                       |
| `target`   | `website` or `game`, the word `list_projects` returns. `kind` still works as another name                                                                                                        |
| `template` | Optional. A website template such as `floating-diorama` (the default), `room` or `character-showcase`, or a game template: `prop-set`, `character`, `level-kit` or `level-kit-obj`               |
| `siteRepo` | Optional. The site repo, or the game repo for a game: a git repo or worktree, absolute or starting with `~`, outside the workspace. Set it later with `set_ship_settings` or `set_game_settings` |

It takes the same path as New project in the app and spends nothing. The app's open project stays as it is unless none was open. No tool removes a project. Only a person does, in the app, which moves it to the workspace trash. A removed project leaves `list_projects`, and a tool given its id fails with not-found, saying it was removed and only a person can restore it.

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

With Ask before shipping on, and always in a [game project](/docs/start/game-projects/), the run stops before Ship and answers `waiting for approval` until you approve or deny it in the app. Poll [`get_run`](#get_run) instead of starting the run again. A website Ship into a repo an agent chose waits whatever that setting says: a run with `repo`, and a project whose site repo an agent set (`create_project`, `set_ship_settings`) until you pick another folder in the app. A project with no site repo never waits: the run leaves Ship out and answers `done` with `shipped: false`, and `next` says to set the repo.

| Parameter     | Type    | Required | Notes                                                                                                                                |
| ------------- | ------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `project`     | string  | yes      | The project id.                                                                                                                      |
| `from`        | string  | no       | A step id. It runs even when cached, and so does everything after it.                                                                |
| `force`       | boolean | no       | Ignore the cache and run every step.                                                                                                 |
| `buildOnly`   | boolean | no       | Stop before Ship: the run builds everything and writes nothing to the site.                                                          |
| `waitSeconds` | number  | no       | How long to wait before answering. 20 by default, 120 at most.                                                                       |
| `repo`        | string  | no       | An absolute path to a git checkout or worktree that this one run ships into instead of the project's repo. Its Ship always asks you. |

```json
{ "project": "moon-island", "from": "compress" }
```

A build only run never waits for approval, since it has no Ship step. Its summary carries `"buildOnly": true`.

### get_run

`read only`

Reads one run's status and the end of its log. The status is `queued`, `running`, `done`, `failed`, `cancelled` or `waiting for approval`. A finished run also says `shipped` (true or false) and `shipReason`: a run in a project with no site repo is `done` with `shipped: false`.

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

Reads a character asset's VRM avatar, as the [Avatar view](/docs/views/avatar/) shows it, as a short answer: the VRM readiness of the version (`mapped` bones of the 55, the 15 `required` and `missingRequired` by name, the `naming` report (the convention the joints matched, ambiguous matches and whether the map is confident), the rest pose measures, the `findings` blockers first and `jointCount`), the spec (the VRM meta, the bone map overrides and the material mode, taken from Settings > Avatar defaults until it is first saved), whether the rest pose needs normalising, the last normalised rest pose with its before and after sheets and its state (`pending`, `approved` or `denied`), `springs`, the last export and `waitingApproval`. The 55 bones with the joint each maps to and the naming it matched by come back under `readiness.bones` only with `include: ["bones"]`, and the skin's joints to pick from under `readiness.joints` with `include: ["joints"]`. `springs.ready` says the model the VRM is written from is ready (its rest pose is VRM's, or its new one is approved); `springs.found` lists the parts that could swing a build found on it, each with an `id` such as `hair-1`, a `kind` (`hair`, `cloth` or `accessory`), the `rootBone` it hangs from, `length`, `vertices`, `confidence` (0 to 1), the `joints` a chain gets by its length and its `color` on `springs.found.sheet`; `springs.chained` is the last chain step with its sheets and state. The confirmed springs are `spec.springs`.

| Parameter | Type   | Required | Notes                                                                           |
| --------- | ------ | -------- | ------------------------------------------------------------------------------- |
| `project` | string | yes      | The project id.                                                                 |
| `asset`   | string | yes      | The asset id or name.                                                           |
| `version` | number | no       | The version to read; the newest otherwise.                                      |
| `include` | list   | no       | `"bones"` and `"joints"` add the 55 bones and the skin's joints to `readiness`. |

### set_avatar

`changes the project`

Changes a character asset's avatar spec in the project's `avatar.json`, as the Avatar view's form does. Only what you give changes.

| Parameter    | Type    | Required | Notes                                                                                                                                                                                                                                                       |
| ------------ | ------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `project`    | string  | yes      | The project id.                                                                                                                                                                                                                                             |
| `asset`      | string  | yes      | The asset id or name.                                                                                                                                                                                                                                       |
| `reset`      | boolean | no       | Start again from the workspace's avatar defaults first.                                                                                                                                                                                                     |
| `meta`       | object  | no       | VRM 1.0 meta fields by name (`name`, `authors`, `commercialUsage`, `allowRedistribution` and the rest); `null` puts one back to its default.                                                                                                                |
| `bones`      | object  | no       | A VRM bone name to the joint it maps to, or `null` to leave the bone unmapped.                                                                                                                                                                              |
| `clearBones` | array   | no       | Bones whose override is dropped, so they map by name again.                                                                                                                                                                                                 |
| `material`   | string  | no       | `mtoon-soft` (toon shading from the base color, no outline) or `pbr`.                                                                                                                                                                                       |
| `highlights` | string  | no       | With `mtoon-soft`: `soften` (the default) tones down strong highlights baked into dark material on a copy of the texture, and leaves a texture alone when more than 5% of it reads as highlights (pale and dark parts sharing one atlas); `keep` leaves it. |
| `bridges`    | string  | no       | `hide` (the default) takes the faces fused between an arm and the body, which stretch when it lifts, out of the reweighted model; `keep` leaves them.                                                                                                       |
| `springs`    | array   | no       | The springs to confirm, replacing the ones before, or `"none"` for no springs; `[]` only clears the picks. See below.                                                                                                                                       |

Each spring names a candidate from `get_avatar`'s `springs.found` by `id`, and takes the rest from it: `{ "id": "hair-1" }`. It can add `joints` (1 to 16), `preset` (`soft` for hair, `heavy` for cloth, `stiff` for small parts; by kind otherwise) and the swing itself: `stiffness` (0 to 4), `gravityPower` (0 to 2), `dragForce` (0 to 1) and `hitRadius` (0 to 0.5 metres). A small piece (`small: true`, listed after the other parts as `charm-1`, `charm-2`: a charm on a belt or a jacket, a short chain, an earring close to the body) keeps its short chain of 2 to 4 joints unless `joints` is given, and swings `stiff` as an accessory. A part the finder did not list takes `kind`, `rootBone` and `line` too. Nothing is confirmed on its own: confirm only the parts a person or you judged from the candidates sheet (a rigid brim looks like hair to the finder), and say which.

`"springs": "none"` confirms that the VRM carries no springs. An empty list is not the same: it clears the picks, and when the last export had springs, the next build answers `candidates waiting for a pick` instead of writing the file without them.

The answer is short: `status` (`saved` or `unchanged`), `changed` (the spec fields that changed, such as `meta.name` or `bones.leftUpperArm`), the saved `spec`, `problems` (what still stops a build: the meta, a readiness blocker) and `ready`. Read `get_avatar` for the rest.

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

`siteRepo` sets a website's site repo, and only null clears it. It must be a git repo or worktree, absolute or starting with `~`, and not the workspace, inside it, around it or the home folder. It is refused while the project has a run going. Everything in the call is checked against the new repo before anything is written. A repo set here counts as an agent's: its dev command asks you before it first runs (see `start_page_preview`), and an agent's run asks you before it ships into it, whatever Ask before shipping says (see `run_pipeline`). A game project is refused: its repo goes through `set_game_settings`, which takes the same checks.

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
| `style`     | object or null | no       | `{ look?, easing?, keys }`. Sets the page's render style track whole, `null` removes it.                          |
| `text`      | object or null | no       | `{ easing?, keys }`. Sets the page's hero text track whole, `null` removes it.                                    |

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

Attaches a download from `needsLook` or `unmatched` to an asset: it goes into the inbox for that asset, as a download named for its send does. A download carrying a recipe bake's mark is refused with `staged-bake` (the watcher never places one by itself either): it becomes a version only through its review in the Motion view. It is handed to the asset's newest waiting By hand send, or, when none waits, put in the inbox aimed at the asset (`list_inbox` shows it as `aimedAt`). The usual checks run there. It answers with the arrival, its `inboxFile` (the inbox id) and the `job` it went to, if any; `accept_inbox_model` with that id then goes to the asset by default and closes the asset's By hand sends still waiting. Look at a `needsLook` download before attaching it: the watcher never attaches one by itself, so the call is the check. The download stays where it is.

| Parameter | Type   | Required | Notes                                      |
| --------- | ------ | -------- | ------------------------------------------ |
| `arrival` | string | yes      | The download's `id`, from `list_handoffs`. |
| `project` | string | yes      | The project the asset is in.               |
| `asset`   | string | yes      | The asset id or name that gets the model.  |

### add_model_file

`changes the project`

Adds a model file you made or fixed yourself as an asset's next version. `path` is an absolute path to a `.glb`, `.gltf`, `.fbx` or `.obj`; a file inside the workspace's inbox or a project's `models` folder is refused, since it is there already, and a staged recipe bake, or any file carrying a recipe bake's mark (one copied out of the cache), is refused with `staged-bake`: it becomes a version only through its review in the Motion view (`add_recipe_bake` asks for one), which keeps whether it counts as Mixamo's. The file is copied, never moved: a `.glb` as it is, the others turned into a GLB by headless Blender first. It goes into the inbox for the asset as `attach_download` does: to the asset's newest waiting By hand send when one waits (accepting closes it), else aimed at the asset. The usual checks run there. It answers with the `inboxFile`, the `job` it went to, if any, and the asset; with `accept` true it is accepted at once and `accepted` carries the new version.

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

Each spend an agent asked for carries `decision`, the answer that let it go (null for a send you started in the app, where the send form was the yes, and for sends from before answers were kept). `decisions` lists the latest answers to approvals of every kind, denied, cancelled and expired ones too, newest first. An answer has `atMs`, `decision` (`approved`, `denied`, `expired` or `cancelled`), `kind` (`send-to-3d`, `image-round`, `ship`, `page-preview`, `rest-pose`, `reweight`, `springs` or `rig-doctor`), `approval` and `requestedAtMs`, what it covered (`project`, `jobs`, `conceptJobs`, `runId`, `provider`, `asset`, `estimate`, `estimateUsd`) and `by`: `source` (`floating-card`, `agents-panel`, `ship-view`, `avatar`, `rig-doctor`, or for an answer no one gave `jobs-cancelled`, `run-cancelled`, `question-changed` or `quit`), `input` (`pointer` or `keyboard`), `trusted` (the browser's word for the click, false when a script made it; true does not prove a person pressed it), `pageFocused` and `pageVisible`, all as the app's own page reported them, and `windowFocused`, `windowVisible` and `windowMinimized`, read from the operating system. `source` also takes `workspace-changed`. `get_provider_job` carries the same `decision` for its send.

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

Copies an asset's newest model, picture, refs, real size, fix-ups and material tweaks into the library, as Add to library does. The same model again (the same hash) adds nothing, and a newer model of an asset already there adds the next version. The recipe bakes the asset took in go with the version, so a project that uses it records them and a Mixamo bake's clips stay out of its packs, even when the file lost the bake's mark; while the project's `recipe-bakes.json` cannot be read the add is refused. It spends nothing and the app updates live.

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

A pack an agent asks for is written inside the workspace only, to `exports/<name>` by default, with `README.txt`, `manifest.json` (names, versions, triangles, source and licence notes) and `contact-sheet.png`, and zipped next to the folder once every model passes. A warning or a failure waits for [`decide_pack`](#decide_pack). With `dryRun`, it starts nothing and answers the plan: the models, the biggest model per kind against its budget, about what the zip weighs, how many models lose Mixamo clips and `warnings`, which names a project whose `recipe-bakes.json` cannot be read (its models' clips then stay out, in case one is a recipe bake of a Mixamo clip). It spends no credits.

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

Answers what a pack asks for. `raise` (with `key` and `triangles`) exports a model again at a higher budget, `keep` (with `key`) lets a warned model into the pack, `again` (with `key`) exports a model again as it is. `zip-passing` zips the models that pass and leaves the rest out, `zip-all` zips every model with files. `stop` stops after the model running now and `resume` goes on with a stopped pack from the model it reached. On a stopped pack `keep` only records the answer, and `raise` or `again` runs just that one model, then the pack is stopped again until `resume`. On a running or waiting pack an answer works as before. It answers the pack as `get_pack` does.

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

### render_styles

`writes files`

Renders a project's models or library entries in headless Blender, through the pipeline's runner, one file at a time, into `exports/renders/<asset>/`. No credits. It answers at once with a render id; follow it with `get_render`. Give `project` (with `assets` for some of them) or `entries`. A loop is a WebP by default and needs nothing beyond Blender. `gif` and `mp4` need ffmpeg, and a GIF always keeps the fixed rules: one palette, 20 fps, Bayer dithering, a crop box. Toon is left out for a model with no texture, with a problem that says so.

| Parameter  | Type     | Required | Notes                                                                                                                                                                                                                                                      |
| ---------- | -------- | -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `project`  | string   | no       | A project id.                                                                                                                                                                                                                                              |
| `assets`   | string[] | no       | Asset ids in the project. Every accepted model by default.                                                                                                                                                                                                 |
| `entries`  | string[] | no       | Library entry ids.                                                                                                                                                                                                                                         |
| `styles`   | string[] | no       | Any of `blueprint`, `toon`, `clay`, `shaded`, `turntable`, `contact-sheet`. `clay` by default.                                                                                                                                                             |
| `settings` | object   | no       | Fields to change: `camera`, `size`, `transparent`, `lineColor`, `paperColor`, `lineWeight`, `crease`, `hiddenDashed`, `marks`, `titleBlock`, `clayColor`, `format`, `length`, `width`, `look`, `tilt`, `stillBackground`, `tightCrop`, `columns`, `names`. |
| `sheet`    | boolean  | no       | Also one contact sheet of all the models.                                                                                                                                                                                                                  |
| `replace`  | boolean  | no       | Write over earlier renders instead of numbering new files.                                                                                                                                                                                                 |
| `dryRun`   | boolean  | no       | List the models and where their files would go; start nothing.                                                                                                                                                                                             |

```json
{ "project": "frostveil", "assets": ["crystal-lantern"], "styles": ["blueprint", "turntable"] }
```

### get_render

`read only`

Reads one render: its `state` (`running`, `stopped` or `done`), how many files are done, and each model's files with their path, `state` (`queued`, `running`, `done`, `warn`, `failed` or `skipped`), bytes, seconds, a turntable's moving area and a problem with what to do next.

| Parameter | Type   | Required | Notes                                                         |
| --------- | ------ | -------- | ------------------------------------------------------------- |
| `id`      | string | yes      | The render id, as `render_styles` or `list_renders` gives it. |

### list_renders

`read only`

Lists the newest renders. A render Maquette closed on reads as `stopped`, and `decide_render` `resume` goes on with it.

### decide_render

`writes files`

`stop` stops a render after the file running now; `resume` goes on with a stopped render from the next file.

| Parameter | Type   | Required | Notes               |
| --------- | ------ | -------- | ------------------- |
| `id`      | string | yes      | The render id.      |
| `choice`  | string | yes      | `stop` or `resume`. |

### render_views

`writes files`

Renders one asset version from a fixed set of views in headless Blender, in [the page light](/docs/pipeline/page-light/), so a rebuild can be held beside its concept ref without a render script. The model may be built or from a provider. Five 1024 px PNGs:

| View            | Where the camera stands                                                                             |
| --------------- | --------------------------------------------------------------------------------------------------- |
| `three-quarter` | 38 degrees round from the front towards the model's +X and 22 up, the view object refs are drawn at |
| `front`         | Straight in front, on the model's +Z side, 6 degrees up                                             |
| `top`           | Straight above, the front at the bottom of the picture                                              |
| `close-up-1`    | From the three quarter side: the first half of the model along its longest side                     |
| `close-up-2`    | The other half                                                                                      |

They land in the project's `exports/renders/<asset>/views/v<n>/` (a set with another `angle` or other `closeUps` gets a folder of its own, `v<n>-<8 hex>/`, so it never overwrites the standard set), with `sheet.png` (every picture on one PNG, the asset's refs first) and `views.json`. The answer gives each picture's path, the camera that took it in words and numbers, what it frames, and the paths of the asset's refs with their views. The asset's fix-ups and real size are applied first, as Build scene places the model, so the pictures are in metres.

`compare` renders a second version with the very same cameras, fitted to both models together, into `v<n>-vs-v<m>/` as `v3_front.png` and `v2_front.png`: a rebuild beside the model it replaces.

A set that is already there answers at once. A render takes about a minute; past `waitSeconds` the answer has `status` `rendering`, and the same call again joins it. No credits, no approval.

| Parameter     | Type     | Required | Notes                                                                                                                                                                                      |
| ------------- | -------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `project`     | string   | yes      | The project id.                                                                                                                                                                            |
| `asset`       | string   | yes      | The asset id or its name.                                                                                                                                                                  |
| `version`     | number   | no       | The version to render. The newest by default.                                                                                                                                              |
| `compare`     | number   | no       | Another version, rendered with the same cameras.                                                                                                                                           |
| `angle`       | object   | no       | `{ azimuth, elevation? }` in degrees moves the three quarter view (and the close ups) to match a ref drawn from elsewhere. A negative azimuth stands on the model's other side.            |
| `closeUps`    | object[] | no       | Up to two of `{ node }` (an object of the model by name), `{ box }` (two corners in shares of the bounding box, x left to right, y bottom to top, z back to front) or `{ half }` (0 or 1). |
| `asIs`        | boolean  | no       | Render the model files as they are, without fix-ups and real size.                                                                                                                         |
| `cancel`      | boolean  | no       | Stop the render of this set that is running.                                                                                                                                               |
| `waitSeconds` | number   | no       | How long to wait for Blender, 60 by default and 240 at most.                                                                                                                               |

```json
{ "project": "frostveil", "asset": "book-stack", "compare": 1, "closeUps": [{ "node": "mug" }] }
```

### get_model_facts

`read only`

Says what an asset version measures and where it stands, so nobody writes a measuring script. It reads the model file and the project's own data, and never runs Blender:

- `counts`: triangles, vertices, meshes, `objects` (the nodes a page can move by name), primitives and materials. One object means a single mesh, whose parts have to be read off pictures.
- `inFile`: the model file's own box. A provider's scan is not at real size in the file.
- `realSize`: what `prepare.json` holds, the scale Build scene gives the file to reach it and the origin fix-up's shift. `atRealSize` is the box the layout places, in metres.
- `facing`: the axis the front points along in the file and, turned by the layout, in the scene, with `how` it was decided (`rig`, read from the skeleton, or `convention`) and how `sure` it is (`measured` or `assumed`). A prop carries no front, so its facing is the format's and the answer says nobody measured it: check it on `render_views`' front picture.
- `layout`: the entry in `layout.json` (position, rotation in degrees, scale) and the box it fills in the scene.
- `refs`: the asset's ref images with their whole paths and views.
- `notes`: what to make of the numbers.

`axes` says how to read them: glTF and three.js share one frame, y up and the front at +z, which Blender imports as -y with z up.

| Parameter | Type   | Required | Notes                                          |
| --------- | ------ | -------- | ---------------------------------------------- |
| `project` | string | yes      | The project id.                                |
| `asset`   | string | yes      | The asset id or its name.                      |
| `version` | number | no       | The version to measure. The newest by default. |

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

Saves a rigged model's clips to the animation library as a pack, as Save clips to library does: a project asset's newest version (`project` and `asset`), or a model waiting in the Inbox (`inbox`, its file name), such as a Tripo job's animations. Only the skeleton and the clips' keys go in. Clips that came from the library are never saved back, nor a recipe bake of a Mixamo clip, and keys the library has are skipped. While the project's `recipe-bakes.json` cannot be read (broken, or from a newer Maquette), nothing is saved and the call is refused with `bakes-unreadable`, since any clip may be such a bake; a person fixes or removes the file. Runs Blender headless and spends nothing.

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

A model's last retarget review: each library clip baked onto it with its status, a one line note, its planted foot slide (`slideCm`, and `unpinnedCm` before Pin the feet when it was pinned, or `pinWorseCm` when the pin slid more and was left off), the bones driven and left at rest, and the version it made, with every library clip baked into the model and the revision it was baked at.

| Parameter | Type   | Required | Notes           |
| --------- | ------ | -------- | --------------- |
| `project` | string | yes      | The project id. |
| `asset`   | string | yes      | The asset id.   |

### list_recipes

`read only`

Lists a project's [animation recipes](/docs/guides/animation-recipes/), one file each in `recipes/<id>.recipe.json`: id, name, the base clip's name and frame count, `mixamo` when the base clip is Mixamo's, the number of `changes` and `blanks`, how many blanks are still `open`, when it last changed and its `version` (send it as `expected` to `update_recipe` or `bake_recipe`). A prop blank whose asset the project no longer has counts as open, with a line in `warnings`. A file that cannot be read lists with its `problem`. An empty list carries a `next` list.

| Parameter | Type   | Required | Notes           |
| --------- | ------ | -------- | --------------- |
| `project` | string | yes      | The project id. |

### get_recipe

`read only`

Reads one recipe: its name, `revision`, and `base` with the clip's name, frames, seconds and hash, plus `rest: true` when the recipe runs on the model's rest pose with no clip, and `library`: `found`, `hash-differs`, `missing` or `rest`, as matched in this computer's animation library by name and frame count (by name alone when the hash is empty, as for a starter's clip), with the matching `clip` id and a `warning` when the match is not exact. A different hash is only a warning: the clip may have been imported again. `mixamo` says a bake will count as Mixamo's (kept out of export packs and never saved to the clip library); the recipe file itself stays shareable. Then its `blanks` (targets filled per project) and its `changes` in the order they are laid over the clip. A prop blank whose asset the project no longer has (removed, or a recipe synced in from another computer) reads as open, with a line in `warnings`. `version` is a hash of the file's bytes, which `update_recipe` takes back as `expected`.

| Parameter | Type   | Required | Notes                                      |
| --------- | ------ | -------- | ------------------------------------------ |
| `project` | string | yes      | The project id.                            |
| `recipe`  | string | yes      | The recipe id, as `list_recipes` gives it. |

### create_recipe

`changes the project`

Makes a recipe on a clip of the animation library (`clip`), or on the model's rest pose with no base clip (`restPose: true`: 60 frames at 30 fps, never Mixamo's), one of the two, as the Recipes tab in the Motion view does. It writes one file holding the base clip's name, frames, length and hash, never keys, a mesh or a skeleton, and starts with no changes or blanks. A recipe on a Mixamo clip is marked `mixamo`. Website and game projects both take recipes. Nothing is baked and nothing is spent; `bake_recipe` bakes it. A [starter recipe](/docs/guides/animation-recipes/#starters) can be used instead, with `use_recipe`.

| Parameter  | Type    | Required   | Notes                                                       |
| ---------- | ------- | ---------- | ----------------------------------------------------------- |
| `project`  | string  | yes        | The project id.                                             |
| `clip`     | string  | one of two | The library clip id, as `list_animations` gives it.         |
| `restPose` | boolean | one of two | True starts on the model's rest pose, with no base clip.    |
| `name`     | string  | no         | One line, up to 120 characters. The clip's name by default. |

### update_recipe

`changes the project`

Changes a recipe's name (the file and id stay), its changes, its blanks, or any of them. `changes` and `blanks` each replace the list whole, so read `get_recipe` first and send every entry you keep. Everything is checked and lands in one write, and the revision does not move. Blanks you send must name assets of the project. When you leave `blanks` out, the blanks stay as they are and the change is never refused for them: a prop blank whose asset is gone keeps its prop in the file, so restoring the asset fills it again, and reads as open with a line in `warnings`. A prop blank you send open whose stored prop is such a gone asset keeps it the same way, since it only read as open; name it in `clear` to empty it for good. Sending `changes`, `blanks` or `clear` needs `expected`, the `version` `get_recipe` answered with (a hash of the file's bytes); without it the call is refused (`needs-version`). A recipe that a person, another agent, a hand edit or a sync changed since is refused (`moved`) with nothing written: read it again, keep what changed and resend. A name only update may leave `expected` out. A change without an id gets the lowest free `l-N`; the Motion view gives the entries a person adds random ids (`l-k3x9q`), so the two never meet. Answers with the recipe as `get_recipe` reads it.

A change is `{ id?, kind, bone, weight?, start?, end?, target?, rotation?, offset?, cycles?, amount? }`. Bones are VRM humanoid names. `start` and `end` are shares of the clip from 0 to 1, and `weight` is 0 to 1 (1 by default).

| Kind      | Bones                                                                                     | Settings                                                                                   |
| --------- | ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `reach`   | `leftHand`, `rightHand`, `leftFoot`, `rightFoot`                                          | `target`: a blank's id. The limb reaches it with IK.                                       |
| `look-at` | `head`, `neck`                                                                            | `target`: a blank's id. The head or neck turns to follow it.                               |
| `nudge`   | any humanoid bone                                                                         | `rotation` in degrees (within 180 each way) and or `offset` in metres (within 2 each way). |
| `wave`    | `leftUpperArm`, `rightUpperArm`, `leftLowerArm`, `rightLowerArm`, `leftHand`, `rightHand` | `cycles` (more than 0, at most 50) and `amount` in degrees (more than 0, at most 90).      |

A blank is `{ id, label, kind, prop?, point? }`: `kind` is `prop` (an asset of the project, by id) or `point` (`[x, y, z]` in metres from where the model stands, in its own frame: x to its left, y up, z the way it faces). Left empty, it stays open for the person or another agent to fill.

| Parameter  | Type     | Required   | Notes                                                            |
| ---------- | -------- | ---------- | ---------------------------------------------------------------- |
| `project`  | string   | yes        | The project id.                                                  |
| `recipe`   | string   | yes        | The recipe id.                                                   |
| `name`     | string   | no         | A new name.                                                      |
| `changes`  | object[] | no         | Every change, in order. Replaces them.                           |
| `blanks`   | object[] | no         | Every blank. Replaces them.                                      |
| `expected` | string   | with lists | The `version` read. Needed with `changes`, `blanks` or `clear`.  |
| `clear`    | string[] | no         | Prop blank ids to empty for good, even when their asset is gone. |

```json
{
  "project": "frostveil",
  "recipe": "standing-idle",
  "blanks": [
    {
      "id": "lantern",
      "label": "The prop the hand reaches",
      "kind": "prop",
      "prop": "crystal-lantern"
    }
  ],
  "changes": [
    { "kind": "reach", "bone": "rightHand", "target": "lantern", "start": 0.2, "end": 0.6 }
  ]
}
```

### set_recipe_keys

`changes the project`

Edits the keys of one pose key change, as the Motion view's pose keys do, without sending the whole change list. `layer` names the change; left out, or an id the recipe has not got, a new pose key change is added at the end. `remove` goes first, then `move`, then `set`, and a key left with no bones goes. Each `set` entry is `{ clip?, frame, ease?, pose }`: the key on that clip's own frame (clip 0 the base, then the joined clips; on the rest pose the recipe's own frame), turning each bone in `pose` to degrees [x, y, z] about the model's own axes (X to its left, Y up, Z ahead) under its parent. `ease` (`smooth`, `in`, `out`, `linear`, `step`) is how the motion eases into the key. `mode` is `add` (on top of the clip, the default) or `replace` (from the rest pose), and `weight` 0 to 1. It needs `expected`, the version `get_recipe` read; a recipe changed since is refused (`moved`). Answers the change, its key count and bones, and `overLimits`: the keyed turns past the humanoid joint limits, which only warn. Pose keys bake.

| Argument   | Type   | Required | What it is                                                                 |
| ---------- | ------ | -------- | -------------------------------------------------------------------------- |
| `project`  | string | yes      | The project id.                                                            |
| `recipe`   | string | yes      | The recipe id.                                                             |
| `expected` | string | yes      | The `version` `get_recipe` read.                                           |
| `layer`    | string | no       | The pose key change's id; left out, a new one is added.                    |
| `mode`     | string | no       | `add` or `replace`.                                                        |
| `weight`   | number | no       | 0 to 1.                                                                    |
| `set`      | array  | no       | `{ clip?, frame, ease?, pose }` per key.                                   |
| `move`     | array  | no       | `{ clip?, frame, toClip?, toFrame, bones? }`: a key, or some bones, moved. |
| `remove`   | array  | no       | `{ clip?, frame, bones? }`: a key, or some bones, taken out.               |

```json
{
  "project": "frostveil",
  "recipe": "slash-with-a-lean",
  "expected": "v3",
  "set": [{ "clip": 0, "frame": 20, "ease": "out", "pose": { "chest": [12, 18, 0] } }]
}
```

### get_recipe_pose

`read only`

Reads the turns a recipe's pose keys make at one frame of its bake, from the same math the rough preview and the bake blend them with: the clip and its own frame that frame shows, and per keyed bone its pose key change, `mode`, `weight`, `turn` in degrees, its joint `limits` per axis (none for the hips) and each axis `over` them. An add key's turn sits on the clip's own, which is not counted. `layer` reads one pose key change only.

| Argument  | Type   | Required | What it is                          |
| --------- | ------ | -------- | ----------------------------------- |
| `project` | string | yes      | The project id.                     |
| `recipe`  | string | yes      | The recipe id.                      |
| `frame`   | number | yes      | A frame of the bake, 0 to `frames`. |
| `layer`   | string | no       | Only this pose key change.          |

### bake_recipe

`writes files`

Bakes a recipe onto a rigged model of its project in headless Blender, as Apply in the [Motion view](/docs/guides/animation-recipes/) does, and answers at once with the bake's record; follow it with `get_recipe_bake`. The base clip goes onto the model through the same retarget as `use_animations`, then the recipe's changes are solved over it exactly, frame by frame, into one new clip named after the recipe: a reach by IK to its target, a look-at turned the shortest way and held within 80 degrees, a nudge and a wave eased in and out as the rough preview eases them. The facing is read from the rig, as the preview reads it. A prop target sits where the scene layout places the prop, seen from where it places the model, when both are placed; otherwise it takes the preview's spot in front of the model.

The model is staged in the workspace cache, not added to the project: `add_recipe_bake` asks a person to review the bake and add it as the model's next version in the app. The clip is named after the recipe; every clip an earlier bake of this recipe added to the model is replaced, whatever its name, and a same-named clip from anywhere else stays while the new one takes the name with `_2`. A recipe with open blanks is refused (`open-blanks`) naming them; fill them with `update_recipe`. A recipe with no changes, a base clip missing from the library, a model with no rig or a bone the model lacks is refused with what to fix. A bake of a Mixamo base counts as Mixamo's: its model carries a mark, which bringing the file in as a version records on the asset (inbox accept, a build, a library entry or a game import; so it outlives later Blender steps), and so an export pack leaves its clips out and `save_model_clips` never puts its clip in the library, and `add_model_file` refuses the staged file (`staged-bake`). One bake per recipe runs at a time, and a new one replaces the staged one before it. Spends no credits.

| Parameter  | Type   | Required | Notes                                                                      |
| ---------- | ------ | -------- | -------------------------------------------------------------------------- |
| `project`  | string | yes      | The project id.                                                            |
| `recipe`   | string | yes      | The recipe id.                                                             |
| `asset`    | string | yes      | The rigged model to bake onto, an asset id of the project.                 |
| `expected` | string | no       | The `version` `get_recipe` read. Given, a recipe changed since is `moved`. |

### get_recipe_bake

`read only`

Reads a recipe's last bake, or `bake: null` when it was never baked: `state` (`running`, `done`, `failed` or `cancelled`), `startedBy`, `progress` while it runs, `error` when it stopped, and when done `result`: the `recipeVersion` and `baseHash` it was baked from, `mixamo`, the new `clip` and its `frames`, the `facing` read from the rig, `slideCm` (the planted feet's worst slide, beside `baseSlideCm`, the base clip's own on this model), `stretchMm` (the rig's worst stretch past a healthy skin, beside `baseStretchMm`), and per reach its `gapCm` to the target at its end (`reached` within 2 cm) and the first frame its elbow or knee flips, with the timings, and `clipNote`, whether the clip replaced this recipe's own older ones or took a `_2` name. `review` holds each number against its limit as `pass`, `warn` or `not-measured`, with how many warn: a hand or foot gap of 2 cm, no elbow or knee flip, a foot slide up to 1.5 cm over the base clip's own and a rig stretch up to 5 mm over the base clip's. A warn asks for a look and never stops an add. `stale` says why a finished bake is out of date: the recipe changed since, or the model has another newest version than the one it went onto; bake again. While a person has an `add_recipe_bake` of it to answer, `status` is `waiting for approval` with the `approval` id; once added, `state` is `added` with `addedVersion`. The staged file's path is not given: only the review in the app adds it. A bake the app closed on reads as failed.

### add_recipe_bake

`writes files` `needs approval`

Asks a person to add a recipe's finished bake as its model's next version, as the Rig doctor's Add as vN asks: nothing is added by the tool. It answers `status: "waiting for approval"` with the `approval` id, `pollEverySeconds` and `whereToApprove`: the approval card in the app, whose Review opens the bake in the [Motion view's review](/docs/guides/animation-recipes/#add-as-vn), where the person plays it next to the base clip, reads each number against its limit and presses Add as vN. Poll `get_recipe_bake`: it reads as waiting while the person has it, then `added` with `addedVersion`. A Deny keeps the staged bake. Adding takes the same route as the Rig doctor's add (the Inbox for the asset, then accept), records on the new version in `recipe-bakes.json` which recipe and recipe version made it, and that it is Mixamo's for a Mixamo base, and clears the staged model. A bake that is out of date is refused with `stale-bake`: bake it again with `bake_recipe`. A bake still running, failed, cancelled or added already is refused too. Asking twice returns the same approval, and a new bake of the recipe ends it. Spends no credits.

| Parameter | Type   | Required | Notes           |
| --------- | ------ | -------- | --------------- |
| `project` | string | yes      | The project id. |
| `recipe`  | string | yes      | The recipe id.  |

| Parameter | Type   | Required | Notes           |
| --------- | ------ | -------- | --------------- |
| `project` | string | yes      | The project id. |
| `recipe`  | string | yes      | The recipe id.  |

### list_library_recipes

`read only`

Lists the [recipe library](/docs/guides/animation-recipes/#the-recipe-library): recipes kept outside any project in `library/recipes` at the workspace root. Each entry has its `id`, `name`, `latest` revision, every revision with where it came from (`project` and `recipe` it was saved from, or the file it was `imported` from, and `savedAt`), the newest revision's `base` clip, `frames`, `mixamo`, `layers` and `blanks` counts, and its `version`. An entry that cannot be read lists with its `problem`. `starters` lists the [starter recipes](/docs/guides/animation-recipes/#starters) built into the app, read only: each has `starter: true`, an `id` such as `starter:wave-hello`, its `name`, what it does (`about`), its `base` (clip names tried in order, or the rest pose) with `library` as `found`, `missing` or `rest` on this computer, and its change and blank counts. Listing writes nothing.

### get_library_recipe

`read only`

Reads one library recipe: its name, `version`, every revision with where it came from, and one revision in full (the newest unless `revision` is given): its name, `base` with `library` (`found`, `hash-differs`, `missing` or `rest` in this computer's clip library), `mixamo`, `blanks` and `changes`. A revision never changes once it is saved. A starter id (`starter:...`) reads that starter in full instead: its base by name or rest, its blanks and its changes.

| Parameter  | Type   | Required | Notes                                                        |
| ---------- | ------ | -------- | ------------------------------------------------------------ |
| `entry`    | string | yes      | The library recipe's id, as `list_library_recipes` gives it. |
| `revision` | number | no       | A revision number. The newest by default.                    |

### use_recipe

`changes the project`

Copies a library recipe's revision (the newest by default) into a project as a new recipe, as Use in project in the Animations library does. The project recipe remembers the library recipe and revision, so `get_recipe` says when the library has a newer one (`library.newer`). Point blanks keep their points. A prop blank whose asset the project does not have opens, and `notes` says which. The base clip is matched in the clip library by name and frame count: when it is missing the recipe is still made and says so, and another hash only warns. A taken id gets a fresh one (`reach-out-2`). A starter id (`starter:...`) copies that starter instead, with no library link: its base is the first clip of the clip library with one of its names, whatever its frame count, or the first name alone (an empty hash) when none is there, which reads as missing; a rest pose starter needs no clip. `revision` must be left out for a starter. Agents cannot copy a starter to the library: use it in a project, then `propose_recipe_save`. Website and game projects both take recipes.

| Parameter  | Type   | Required | Notes                                                |
| ---------- | ------ | -------- | ---------------------------------------------------- |
| `project`  | string | yes      | The project it is copied into.                       |
| `entry`    | string | yes      | The library recipe's id.                             |
| `revision` | number | no       | A revision number. The newest by default.            |
| `name`     | string | no       | The project recipe's name. The library's by default. |

### propose_recipe_save

`needs approval`

Asks a person to save a project's recipe to the recipe library: as the next revision of the library recipe it came from (or of `entry`), or as a new library recipe (`newEntry`, or when it came from none). Nothing is saved by the tool. It answers `status: "waiting for approval"` with the `approval` id, `whereToApprove` and the `plan`, which says in plain lines what changes against the newest revision. The approval card's Review opens the save to library sheet in the [Motion view](/docs/guides/animation-recipes/#save-back), where the person reads the same lines and saves. A save always adds a revision and never overwrites one; an entry that moved since the sheet opened is refused there. A recipe that holds what the newest revision holds is refused. Asking twice returns the same approval. The approval ends as approved when the person saves where the agent asked, and as cancelled when they save it elsewhere (another entry, or a new one). Poll `get_recipe`: `saveWaiting` holds the approval while it waits, and once saved `library` names the revision.

| Parameter  | Type    | Required | Notes                                                                     |
| ---------- | ------- | -------- | ------------------------------------------------------------------------- |
| `project`  | string  | yes      | The project id.                                                           |
| `recipe`   | string  | yes      | The project's recipe id.                                                  |
| `entry`    | string  | no       | The library recipe to add a revision to. The one it came from by default. |
| `newEntry` | boolean | no       | True proposes a new library recipe even when the recipe came from one.    |

### import_recipe

`changes the project`

Imports a `.recipe.json` into a project as a new recipe. Agents never import into the recipe library: without `project` the call is refused with the code `library-import`, which points to [`propose_recipe_save`](#propose_recipe_save). Import into a project, then propose the save, and the person saves it in the app. Agents import only files inside the workspace: `path` is relative to the workspace root or absolute under it, and a file elsewhere is refused (the person imports those with the app's file picker). The file is untrusted: its size is checked before it is read (at most 256 KB), then it has to be a version 1 recipe with at most 32 changes and 16 blanks that passes every change and blank check. Nothing in it becomes a path: the id is a slug of the file's name, fresh when it is taken, and a library link in the file is dropped. The base clip is matched by name and frame count; another hash warns and a missing one is noted. Prop blanks whose asset the project lacks open. Answers with `notes`.

| Parameter | Type   | Required | Notes                                                           |
| --------- | ------ | -------- | --------------------------------------------------------------- |
| `path`    | string | yes      | The file, relative to the workspace root or absolute inside it. |
| `project` | string | yes      | The project to import into. Never the library.                  |

### export_recipe

`writes files`

Writes a recipe to one `.recipe.json`: a project's recipe (`project` and `recipe`) or a library recipe's revision (`entry`, and `revision`, the newest by default). The file holds only the base clip's name, frames, length, hash and Mixamo flag, the blanks and the changes, never keys, a mesh, a path or the library link, so it can be shared even when its base clip is Mixamo's. Agents write only new files under `exports/recipes` in the workspace: `file` names it (no folder), and a taken name gets `-2`. The person can export anywhere from the app. Answers with the `path` relative to the workspace.

| Parameter  | Type   | Required | Notes                                          |
| ---------- | ------ | -------- | ---------------------------------------------- |
| `project`  | string | no       | With `recipe`: a project's recipe.             |
| `recipe`   | string | no       | The project's recipe id.                       |
| `entry`    | string | no       | Or a library recipe's id.                      |
| `revision` | number | no       | Its revision. The newest by default.           |
| `file`     | string | no       | The file's name. The recipe's name by default. |

### start_rig_doctor

`read only`

Opens the rig doctor's case of a rigged model and reads the file at once, with no Blender: the route (humanoid for Mixamo names, creature otherwise), what it refuses and why, hand and arm weight far from the limb, and whether it is one continuous mesh. An Inbox model must be accepted with `accept_inbox_model` before it can run. See [Rig doctor](/docs/views/rig-doctor/).

| Argument  | Type   | Required | What                                              |
| --------- | ------ | -------- | ------------------------------------------------- |
| `inbox`   | string | no       | An Inbox item id. Or give project and asset.      |
| `project` | string | no       | A project id.                                     |
| `asset`   | string | no       | An asset id in that project.                      |
| `version` | number | no       | The version to work from. Defaults to the newest. |

### run_rig_doctor

`writes files`

Starts a case's step in headless Blender on a copy and answers at once; follow it with `get_rig_doctor`. It takes minutes and spends no credits. `pose-test` renders the sheets and names the defects, `fix` changes joints and weights only and verifies that, and `round-2` is one last try. A third round is refused. The draft waits for a person in the app, on an approval card: no tool adds it or approves a joint move.

| Argument | Type   | Required | What                             |
| -------- | ------ | -------- | -------------------------------- |
| `id`     | string | yes      | The case id.                     |
| `step`   | string | yes      | `pose-test`, `fix` or `round-2`. |

### get_rig_doctor

`read only`

A case's state, model, refusals, defects by place, creature chains, before and after sheets with each pose's stretch and seam gap in mm, fix steps, checks, the joint move if one is asked with the clips it would break, progress, log and why it stopped. Look at the sheet files: the numbers say where to look.

| Argument | Type   | Required | What         |
| -------- | ------ | -------- | ------------ |
| `id`     | string | yes      | The case id. |

### list_rig_doctors

`read only`

The cases, newest first: id, name, state, route, round, who started it and the version it landed as.

### list_canvases

`read only`

Lists a project's planning canvases: the id every other canvas tool takes as `canvas`, the name, preset, node and link counts and when it last changed. A canvas file that cannot be read lists with its `problem`.

| Argument  | Type   | Required | What            |
| --------- | ------ | -------- | --------------- |
| `project` | string | yes      | The project id. |

```json
{
  "canvases": [
    {
      "id": "night-story",
      "name": "Night story",
      "preset": "story",
      "nodes": 12,
      "links": 9,
      "edited": "2026-11-02T12:40:00.000Z"
    }
  ]
}
```

### get_canvas

`read only`

Reads a canvas: each node's id, kind, title, text, `url`, picture path, linked asset (with its live name, stage and version count, or `gone`), Lab direction, `x`, `y`, `by` (the agent that added it, absent for a person) and `personWrote: true` on an agent's node a person wrote in (it is theirs now, and stays theirs), and the links between nodes (`from`, `to`, `kind` when it is not `relates`, `label`, `by`). Texts are cut at 120 characters unless `full`. It also answers `autoLayout` (the person's Auto layout after agents switch), `overlaps` on a Game mechanics canvas and `proposals`, the removal proposals still waiting for the person. Read it before `add_nodes` so new nodes go near the right one.

An overlap is two system cards (`a`, `b`) whose `Verb:`, `Look:` or `Input:` lines share a main word (`part`, `word`): the first word of a verb that is not a little word like `to` or `the`, the last of a look, each key of an input with the gesture (`tap`, `hold`) left out, plurals read as singular. `fine: true` when the person marked the pair as fine; that holds until either card's title or text changes, and only the person marks it. A proposal lists its `id`, `by`, `reason`, its `nodes` (each `here`, `changed` or `gone`) and `stale` when any node changed or went.

| Argument  | Type     | Required | What                                                  |
| --------- | -------- | -------- | ----------------------------------------------------- |
| `project` | string   | yes      | The project id.                                       |
| `canvas`  | string   | yes      | The canvas id, as `list_canvases` returns it.         |
| `full`    | boolean  | no       | Whole node texts instead of the first 120 characters. |
| `nodes`   | string[] | no       | Only these node ids, with the links that touch them.  |

```json
{
  "id": "night-story",
  "name": "Night story",
  "preset": "story",
  "nodes": [
    {
      "id": "n-1",
      "kind": "note",
      "title": "Dusk",
      "text": "The light turns violet over the ridge.",
      "x": 0,
      "y": 0
    },
    {
      "id": "n-2",
      "kind": "note",
      "text": "Bells from the church.",
      "x": 340,
      "y": 0,
      "by": "Research agent"
    }
  ],
  "links": [{ "id": "e-1", "from": "n-1", "to": "n-2", "kind": "leads-to", "by": "Research agent" }]
}
```

### get_changes

`read only`

Reads a canvas's change log, newest first. Every write by a person or an agent is logged: its id (what `undo_run` takes as `changes`), time, `by` (an agent's name, absent for a person), `run`, a one line summary and `undone` when it was taken back.

| Argument  | Type   | Required | What                                                         |
| --------- | ------ | -------- | ------------------------------------------------------------ |
| `project` | string | yes      | The project id.                                              |
| `canvas`  | string | yes      | The canvas id.                                               |
| `limit`   | number | no       | The newest this many changes. 20 by default, 200 at most.    |
| `since`   | string | no       | Only changes after this RFC 3339 time.                       |
| `by`      | string | no       | Only changes by this agent name, or `person` for a person's. |

```json
{
  "changes": [
    {
      "id": "c-mh2k1x9a001",
      "at": "2026-11-02T12:40:00.000Z",
      "by": "Research agent",
      "run": "r-1240",
      "summary": "Added 3 notes"
    }
  ]
}
```

### create_canvas

`writes files`

Makes a planning canvas, as Plan > New canvas does. It writes one file, `plans/<id>.plan.json`, and answers with the id (made from the name), the preset and how many nodes and links it starts with.

A preset only sets the start; every canvas holds every kind of node, and the camera plan is built in, never a preset:

- `brainstorm` (the default, `blank` is accepted too): an empty canvas.
- `story`: eight notes and seven links: four beats left to right joined in order, two characters above and two places below, each linked to the beat it first shows in. A game project's notes talk about the player, a website's about the visitor and the scroll.
- `mechanics`: a card per system (core action, movement, defence), each with a verb, a look and an input line, and a note on keeping systems apart.

Preset names are not case sensitive. The starter notes carry your name as `by` and are the canvas's first change, with no run, so the person can undo them all at once from the Changes panel, and you can with `undo_run` and the `change` id this answers with. They are plain guidance for the person to replace. A blank canvas starts with nothing to undo, so its answer has no `change`. When that change cannot be logged the canvas is removed again and the call fails; if the canvas cannot be removed (a file another program holds on Windows), the change is logged again so it stays undoable. An unknown preset is refused with the list, and nothing is written.

| Argument  | Type   | Required | What                                                                                                                |
| --------- | ------ | -------- | ------------------------------------------------------------------------------------------------------------------- |
| `project` | string | yes      | The project id.                                                                                                     |
| `name`    | string | yes      | The canvas name, one line, up to 120 characters.                                                                    |
| `preset`  | string | no       | `brainstorm` (the default), `story` or `mechanics`.                                                                 |
| `agent`   | string | no       | Your name as people see it in the canvas and its log, such as `Research agent`. Defaults to your MCP client's name. |

```json
{ "project": "frost-game", "name": "Act 1", "preset": "story" }
```

```json
{
  "id": "act-1",
  "file": "plans/act-1.plan.json",
  "preset": "story",
  "nodes": 8,
  "links": 7,
  "change": "c-mh8k2f4a001"
}
```

### add_nodes

`writes files`

Adds nodes (`note`, `image`, `link`, `asset` or `direction`) and links to a canvas. A node without `x` and `y` is placed in a column right of the `near` node (or right of everything), clear of the other nodes, never stacked. Everything is checked before anything is written. What it adds carries your name as `by`, shows in the canvas with a blue ring and lands in the change log, so a person can undo it. An open canvas reloads and keeps the person's unsaved edits.

| Argument  | Type     | Required | What                                                                                                                                                                                                                                                      |
| --------- | -------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `project` | string   | yes      | The project id.                                                                                                                                                                                                                                           |
| `canvas`  | string   | yes      | The canvas id.                                                                                                                                                                                                                                            |
| `near`    | string   | no       | A node id the new nodes relate to. Left out, they go right of everything.                                                                                                                                                                                 |
| `nodes`   | object[] | no       | Up to 100. Each has an optional `kind` (it follows the fields when left out), `title`, `text` (20000 characters at most), `url`, `image` (a path relative to the project), `asset`, `direction`, `x` and `y` (both or neither) and `width` (120 to 2000). |
| `links`   | object[] | no       | Up to 100: `{ from, to, kind?, label? }`. `from` and `to` are node ids, or `$0`, `$1` for this call's nodes in order. `kind` is `relates` (the default), `leads-to` or `source`. A pair linked already is left out and noted.                             |
| `agent`   | string   | no       | Your name as people see it. Defaults to your MCP client's name.                                                                                                                                                                                           |
| `run`     | string   | no       | A run id you choose to group several calls, so a person can undo them together.                                                                                                                                                                           |

```json
{
  "project": "moon-island",
  "canvas": "night-story",
  "near": "n-1",
  "nodes": [
    { "title": "Bells", "text": "Bells from the church." },
    { "kind": "link", "title": "Reference", "url": "https://example.com/church" }
  ],
  "links": [{ "from": "n-1", "to": "$0", "kind": "leads-to" }],
  "run": "r-1240"
}
```

```json
{ "added": ["n-7", "n-8"], "links": ["e-4"], "placed": 2, "change": "c-mh2k1x9a001" }
```

### update_nodes

`writes files`

Edits or moves nodes: only the fields given change. On the camera plan (canvas `camera-plan`), `stop` changes a stop node's camera settings; they stay in the plan until the person applies it. Any node may be edited, a person's too, because the change log keeps what it said before and the edit can be undone. When the person edits the same field before saving, they choose in the app which value stays. The log keeps a person's earlier version until your change is released (the person presses Undo or Keep in the Changes panel, or you call `undo_run`), so a call is refused, with nothing written, when its undo record would pass 256 KB (send fewer nodes) or when the agents' changes to a person's content not released yet already hold 4 MB (adding nodes still works, and `undo_run` is never refused for room).

| Argument  | Type     | Required | What                                                                                                              |
| --------- | -------- | -------- | ----------------------------------------------------------------------------------------------------------------- |
| `project` | string   | yes      | The project id.                                                                                                   |
| `canvas`  | string   | yes      | The canvas id.                                                                                                    |
| `nodes`   | object[] | yes      | Up to 100: `{ id, title?, text?, url?, x?, y?, width?, stop? }`, `stop` as `{ name?, position?, target?, fov? }`. |
| `agent`   | string   | no       | Your name as people see it.                                                                                       |
| `run`     | string   | no       | A run id to group several calls.                                                                                  |

```json
{
  "project": "moon-island",
  "canvas": "night-story",
  "nodes": [{ "id": "n-2", "text": "Bells from the church, far away." }]
}
```

```json
{ "updated": 1, "change": "c-mh2k1x9a002" }
```

### remove_nodes

`writes files`

Removes nodes and links you added. A node goes with its links. A person's node or link is refused with the reason, and so is a node of yours a person wrote in (`personWrote` in `get_canvas`) and a node that has a link someone else drew: agents never delete a person's work, so ask the person. Agent names are labels, not accounts: a node made under another agent's name is refused too, but any agent can pass any name, so that is a courtesy, not a protection. Ids no longer on the canvas are noted, not refused.

| Argument  | Type     | Required | What                                                 |
| --------- | -------- | -------- | ---------------------------------------------------- |
| `project` | string   | yes      | The project id.                                      |
| `canvas`  | string   | yes      | The canvas id.                                       |
| `nodes`   | string[] | no       | Node ids you added.                                  |
| `links`   | string[] | no       | Link ids you drew.                                   |
| `agent`   | string   | no       | Your name, a label. What carries it counts as yours. |
| `run`     | string   | no       | A run id to group several calls.                     |

```json
{ "project": "moon-island", "canvas": "night-story", "nodes": ["n-8"] }
```

```json
{ "left": 11, "change": "c-mh2k1x9a003" }
```

### layout_canvas

`writes files`

Tidies the nodes you added: each one that overlaps another node moves to a free spot beside the node it links to. Nodes already clear stay, and a person's nodes, or yours a person wrote in, never move. A node of yours that a person dragged (`personPlaced` in `get_canvas`) never moves either, and naming it is refused with "A person moved the node..."; give an explicit `x` and `y` to move it. Refused when the person turned Auto layout after agents off for the canvas (`autoLayout: false` in `get_canvas`): place nodes with `x` and `y` then. While it is on (the default), `update_nodes` tidies your edited nodes you gave no `x` or `y` the same way, inside the same change, so the person undoes the edit and the move together (`autoLayout.moved` in its answer lists them). An `x` and `y` you give are always kept. Answers with how many moved.

| Argument  | Type     | Required | What                                                 |
| --------- | -------- | -------- | ---------------------------------------------------- |
| `project` | string   | yes      | The project id.                                      |
| `canvas`  | string   | yes      | The canvas id.                                       |
| `nodes`   | string[] | no       | Only these of your node ids. Left out, all of yours. |
| `agent`   | string   | no       | Your name.                                           |
| `run`     | string   | no       | A run id to group several calls.                     |

```json
{ "moved": 2, "change": "c-mh2k1x9a004" }
```

### undo_run

`writes files`

Undoes your own changes on a canvas: a whole `run`, or change ids from `get_changes` or an edit tool's answer, at most 100 per call. It is careful: a field someone changed since stays as they left it, and a node a person edited or linked since is kept with their links and counted in `kept`. A person's change, or one made under another agent name, is refused; names are labels, so only the person's work is protected. The undo is itself a logged change, and it frees the room the log held for those changes. When everything was changed since, nothing changes (`nothingChanged: true`) but the changes count as undone. A change that a later agent change rewrote on top of (the same field of the same node, or the same node or link) is refused, with nothing written, until that later change is undone first; name both, or the run that holds both, to undo them together, newest first. An undo can not be undone itself: to put your change back, make it again. A change the person kept (Keep in the Changes panel) is theirs and is refused, a run holding one too: ask the person, who can undo it there. Answers with how many changes it undid.

| Argument  | Type     | Required | What                                                           |
| --------- | -------- | -------- | -------------------------------------------------------------- |
| `project` | string   | yes      | The project id.                                                |
| `canvas`  | string   | yes      | The canvas id.                                                 |
| `run`     | string   | no       | Undo every change of yours in this run that is not undone yet. |
| `changes` | string[] | no       | Or these change ids.                                           |
| `agent`   | string   | no       | Your name. It must match the name the changes were made under. |

```json
{ "project": "moon-island", "canvas": "night-story", "run": "r-1240" }
```

```json
{ "undone": 1, "change": "c-mh2k1x9a005", "kept": 0, "nothingChanged": false }
```

### propose_removal

`writes files`

Asks the person to remove nodes you may not remove yourself: a person's node, or one of yours a person wrote in. The canvas does not change. The person sees the nodes marked and your reason in the Changes panel and answers Keep it or Remove; a Remove is the person's own change, which they can undo. No tool answers a proposal, your own included, and `undo_run` refuses the ask: only the person decides. When a node's title, text or link changes, or it goes, before the person answers, the proposal is stale and Remove is refused (moving a node keeps it current). Remove also reads the links again and is refused when their count changed since the person saw it. At most 50 proposals wait per canvas, 20 of them under one agent name, 100 nodes each; the person sees how many links a Remove takes with the nodes. `get_canvas` lists the open ones. Remove your own nodes with `remove_nodes`.

| Argument  | Type     | Required | What                                                        |
| --------- | -------- | -------- | ----------------------------------------------------------- |
| `project` | string   | yes      | The project id.                                             |
| `canvas`  | string   | yes      | The canvas id.                                              |
| `nodes`   | string[] | yes      | The node ids to ask about, 1 to 100.                        |
| `reason`  | string   | no       | Why, in a sentence the person reads (up to 300 characters). |
| `agent`   | string   | no       | Your name.                                                  |
| `run`     | string   | no       | A run id to group several calls.                            |

```json
{
  "project": "moon-island",
  "canvas": "hero-ideas",
  "nodes": ["n-island"],
  "reason": "The porch note replaces it."
}
```

```json
{ "proposal": "c-mh2k1x9a006", "summary": "Asked to remove: Old hero idea: the whole island" }
```

### start_from_node

`writes files`

Starts a new canvas from a node: the node and the nodes linked to it are copied, with the links among them and the node at 0,0, as the new canvas's first change, under your name, which you or the person can undo. The source canvas never changes. Asset and Lab direction nodes stay links by id, and copies keep who made them, so a person's note stays theirs. The new canvas takes the source's preset (a brainstorm when the source is the camera plan) and never the camera plan's id. A camera stop cannot start one, and linked stops are left out (`leftOut`).

| Argument  | Type   | Required | What                                              |
| --------- | ------ | -------- | ------------------------------------------------- |
| `project` | string | yes      | The project id.                                   |
| `canvas`  | string | yes      | The canvas the node is on.                        |
| `node`    | string | yes      | The node id to start from.                        |
| `name`    | string | no       | The new canvas's name. Left out, the node's name. |
| `agent`   | string | no       | Your name.                                        |

```json
{ "project": "moon-island", "canvas": "hero-ideas", "node": "n-porch", "name": "Porch scene" }
```

```json
{
  "id": "porch-scene",
  "name": "Porch scene",
  "preset": "brainstorm",
  "nodes": 3,
  "links": 2,
  "change": "c-mh2k1x9a007"
}
```

### get_camera_plan

`read only`

Reads a website project's camera plan without writing anything. With no plan yet it answers the stops in `layout.json` (with `node: null`) and a `note`; the plan is made when the person opens it or by `propose_camera_plan`. `main` is the line Apply would write, each stop as `{ node, id, name, position, target, fov }` in tour order; `lines` are the other lines, an agent's proposal with its `agent`, stops set aside without one. `stale: true` with `outside` (the counts) when `layout.json` changed since the plan was based on it: the person compares or rebases before Apply. `posters` lists the stops with a poster render. A game project has no camera plan and is refused.

| Argument  | Type   | Required | What            |
| --------- | ------ | -------- | --------------- |
| `project` | string | yes      | The project id. |

```json
{ "project": "moon-island" }
```

```json
{
  "canvas": "camera-plan",
  "main": [
    {
      "node": "n-s1",
      "id": "porch",
      "name": "Porch",
      "position": [0, 1.6, 6],
      "target": [0, 1, 0],
      "fov": 35
    }
  ],
  "lines": [],
  "posters": ["porch"],
  "apply": "Only the person applies a camera plan, with Apply in the app."
}
```

### propose_camera_plan

`writes files`

Proposes a whole tour, the stops in order, as your own line of stop nodes below the person's main line on canvas `camera-plan`. A stop the plan or `layout.json` knows takes only the settings you give; a new stop needs `name`, `position` and `target`. Your earlier proposal under the same name is replaced, except stops a person wrote in or linked (`kept`). It never touches `layout.json` or the main line: the person can take the proposal's order in the app, then reviews the diff and presses Apply. No tool applies a camera plan. Answers with the new node ids, the change id (for `undo_run`) and `vsLayout`, how the proposal compares with `layout.json`.

| Argument  | Type     | Required | What                                                               |
| --------- | -------- | -------- | ------------------------------------------------------------------ |
| `project` | string   | yes      | The project id.                                                    |
| `stops`   | object[] | yes      | 1 to 24, in tour order: `{ id, name?, position?, target?, fov? }`. |
| `agent`   | string   | no       | Your name as people see it, such as "Camera agent".                |
| `run`     | string   | no       | A run id to group several calls.                                   |

```json
{
  "project": "moon-island",
  "agent": "Camera agent",
  "stops": [{ "id": "porch" }, { "id": "moon" }, { "id": "bridge", "fov": 28 }]
}
```

```json
{
  "nodes": ["n-p1", "n-p2", "n-p3"],
  "change": "c-mh2k1x9a010",
  "vsLayout": {
    "counts": "0 stops added · 1 moved · 0 removed · 1 changed",
    "stops": [
      { "id": "moon", "from": 3, "to": 2, "moved": true },
      { "id": "bridge", "from": 2, "to": 3, "changed": ["fov"] }
    ]
  },
  "apply": "Only the person applies a camera plan, with Apply in the app."
}
```

### get_apply_preview

`read only`

Reads what Apply would write to `layout.json` from the camera plan's main line now: `counts`, each stop that changes with its place before (`from`) and after (`to`) and the settings that change, `nodes` (the main line), `dropped` (triggers, timeline and path keys that go with a removed stop), `stale` and `problems` that block Apply. The person sees this same diff in the app before Apply, and an agent's `undo_run` on an Apply is refused: only the person undoes it, from the Changes panel.

| Argument  | Type   | Required | What            |
| --------- | ------ | -------- | --------------- |
| `project` | string | yes      | The project id. |

```json
{ "project": "moon-island" }
```

```json
{
  "counts": "0 stops added · 0 moved · 0 removed · 1 changed",
  "stops": [{ "id": "moon", "from": 3, "to": 3, "changed": ["fov"] }],
  "nodes": ["n-s1", "n-s2", "n-s3"],
  "apply": "Only the person applies a camera plan, with Apply in the app."
}
```
