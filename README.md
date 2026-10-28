<div align="center">
  <img src="assets/showcase/hero.webp" alt="Maquette: the Board, the Lab and the Page view of a sample project, stacked on a warm clay gradient" width="100%" />

  <h1>Maquette</h1>

  <p><strong>Concept art in, optimised 3D for the web and games out.</strong></p>

  <p>
    <img src="https://img.shields.io/badge/Platform-macOS%20%7C%20Windows-lightgrey" alt="Platform: macOS and Windows" />
    <img src="https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white" alt="Tauri 2" />
    <img src="https://img.shields.io/badge/Rust-1.94-b7410e?logo=rust&logoColor=white" alt="Rust 1.94" />
    <img src="https://img.shields.io/badge/License-AGPL--3.0-blue" alt="License: AGPL-3.0" />
    <a href="https://github.com/Shironex/maquette/releases/latest"><img src="https://img.shields.io/github/v/release/Shironex/maquette?label=Release" alt="Latest release" /></a>
    <img src="https://img.shields.io/badge/Status-early%20development-orange" alt="Status: early development" />
  </p>

  <p>
    <a href="https://shironex.github.io/maquette/docs/"><strong>Docs</strong></a>
    &nbsp;·&nbsp;
    <a href="https://shironex.github.io/maquette/docs/start/install/"><strong>Install</strong></a>
    &nbsp;·&nbsp;
    <a href="https://shironex.github.io/maquette/docs/agents/mcp-tools/"><strong>MCP tools</strong></a>
    &nbsp;·&nbsp;
    <a href="https://shironex.github.io/maquette/"><strong>Website</strong></a>
  </p>
</div>

---

> [!WARNING]
> Maquette is in early development. Expect breaking changes and bugs, and the workspace format may change between versions. Back up your workspace before you update.

Maquette is a desktop studio I built to take one asset from a concept image to a small, compressed model in a website or game repo. One Board follows every asset through the stages, Blender does the heavy lifting headless in the background, and the numbers that decide whether a page stays fast (triangles, texture memory, transfer size, frame time) sit next to the model the whole way.

There is no account and no cloud project: a workspace is a folder on your disk, the app spawns the tools you already have, and API keys live in the OS keychain.

## What it does

```text
Concept  ->  Refs  ->  3D model  ->  Blender  ->  Web or Export  ->  Ship
```

1. **Concept.** The Lab turns a brief (a prompt and up to 8 reference images) into concept images through the Codex CLI. Star the good ones, or cut one concept into an isolated image per object.
2. **Refs.** Picks go into the project's `refs/` and become assets on the Board.
3. **3D model.** Send the refs to Tripo or Meshy over their API, or make the model on their site yourself (By hand) and let Maquette pick the export up from your Downloads. A Blender script in the project can build a model too.
4. **Blender.** Every model lands in the Inbox first and is checked against the budgets. Accept moves it into the project, where headless Blender builds the scene, applies the real size and fix-ups (decimate, shrink textures, merge, split into parts, origin) and bakes the light.
5. **Web or Export.** For a website, gltf-transform and `toktx` compress each model (meshopt geometry, KTX2 textures, per model presets) and Blender captures a poster per camera stop. For a game, each model is exported as a plain `.glb` or `.obj` at its path in the game.
6. **Ship.** The models, layout and scene data go into the site repo, or into the game's model folder, where Ship only replaces files it wrote and never commits.

## Features

<table>
  <tr>
    <td width="50%"><img src="assets/showcase/board.webp" alt="Maquette: the Board of the Frostveil sample project, with an asset in every column from Concept to Ship" /></td>
    <td width="50%"><img src="assets/showcase/lab.webp" alt="Maquette: the Lab with six concept images of a frozen lake arena at night, one picked and two starred" /></td>
  </tr>
  <tr>
    <td align="center"><sub>Board: every asset of a project on its way from concept to ship.</sub></td>
    <td align="center"><sub>Lab: concept images from the Codex CLI, starred and picked as refs.</sub></td>
  </tr>
</table>

**One board per project.** Each asset is a card in its stage, with its picture, version, triangles, size and state: generating at Tripo, waiting for a By hand export, needs review, cached, done. Cards can be renamed, copied to another project, sent to 3D in a batch or moved to the workspace trash. A project ships to a website (templates: Floating diorama, Room, Character showcase) or to a game, imported from the models the game already has.

**A concept lab.** Directions keep their prompt, references and images. Each image is one Codex job, a few run at a time, and a job that hits a rate limit runs again on its own.

<table>
  <tr>
    <td width="50%"><img src="assets/showcase/inbox.webp" alt="Maquette: the Inbox previewing Elsheria, a rigged character from Frostveil, with its automatic checks on the right" /></td>
    <td width="50%"><img src="assets/showcase/pipeline.webp" alt="Maquette: the Pipeline view mid run, with six steps, the bake settings, the generated artifacts, a live log and the compression presets" /></td>
  </tr>
  <tr>
    <td align="center"><sub>Inbox: each new model checked for triangles, textures, rig and origin.</sub></td>
    <td align="center"><sub>Pipeline: build, bake and export in headless Blender, then compress, capture posters and ship.</sub></td>
  </tr>
</table>

**An inbox that checks every model.** Triangles and texture sizes against the budgets, a Mixamo rig for characters, PBR maps, an origin away from the bottom centre, separate objects merged into one mesh, and hair or tail vertices weighted to finger bones. A check that a fix-up answers offers it as a button; a prop that came as one mesh gets Split into parts. The preview shows the model shaded, coloured by part, as a wireframe, with a UV checker or with its skeleton.

**A pipeline with a cache.** Steps live in `pipeline.json`. A step whose inputs, script, args and tool version are unchanged is skipped, programs are spawned with an args array and never through a shell, and Blender always runs in the background. Starter scripts ship with the app, and the Pipeline view says when a project's copies are older.

<table>
  <tr>
    <td width="50%"><img src="assets/showcase/inspector.webp" alt="Maquette: the Inspector comparing two versions of a scene with a veil gate, ice crystals and a character, with a split slider, the camera stops below and the selected model's numbers on the right" /></td>
    <td width="50%"><img src="assets/showcase/page.webp" alt="Maquette: the Page view showing the sample site's dev page with the frozen lake scene, its readouts and the current ship against this run" /></td>
  </tr>
  <tr>
    <td align="center"><sub>Inspector: two versions of the scene side by side, with each model's numbers and compression.</sub></td>
    <td align="center"><sub>Page: the site's own dev server, with frame time, scene size and draw calls from the page.</sub></td>
  </tr>
</table>

**See what ships before it ships.** The Inspector compares two versions of the scene with a slider, flies between camera stops, shows triangles, textures, GPU memory and transfer per model, and edits compression, materials, the scene layout and paths. Its Parts tab names, joins and hides a model's parts, splits a one mesh model free in Blender or on Tripo or Meshy (baking the texture back when the split drops it), assembles one prop from part models, and puts it before and after a split side by side. The Page view runs the site's own dev server and reads the real numbers from the page, against the current ship.

<table>
  <tr>
    <td width="50%"><img src="assets/showcase/motion.webp" alt="Maquette: the Motion view with a sword draw clip on a rigged character and its skeleton, with its clips and rig check" /></td>
    <td width="50%"><img src="assets/showcase/timeline.webp" alt="Maquette: the scroll Timeline with camera, object state, explode and energize lanes under a preview of the site" /></td>
  </tr>
  <tr>
    <td align="center"><sub>Motion: rigs, clips and what each one weighs on the page.</sub></td>
    <td align="center"><sub>Timeline: camera stops, object states and paths, keyed to the scroll.</sub></td>
  </tr>
</table>

**Motion and scroll.** Motion plays a model's clips with its skeleton, checks the rig, and sets object states (poses of a prop's parts and the transitions between them). The Timeline keys camera moves, states, explode tracks and lit paths to the page scroll; Ship sends it all to the site with the layout.

<table>
  <tr>
    <td width="50%"><img src="assets/showcase/library.webp" alt="Maquette: the Library grid with eight reusable models, filters by tag, source, format and project, and a crystal lantern's versions" /></td>
    <td width="50%"><img src="assets/showcase/agents.webp" alt="Maquette: the Board with two approval prompts from an agent, one to ship the project and one to send an asset to Tripo" /></td>
  </tr>
  <tr>
    <td align="center"><sub>Library: finished models reused across projects, no credits spent.</sub></td>
    <td align="center"><sub>Agents: Claude Code or Codex over MCP; shipping and paid sends wait for your approval.</sub></td>
  </tr>
</table>

**Refs by view.** Each ref is tagged front, left, back, right or other on its asset, and every send goes in that order. Codex draws the missing views from the front or cleans a busy background, as candidates you pick from, and Send to 3D warns about small, cropped or cluttered refs before any credit is spent.

**Libraries across projects.** Finished models live outside every project, with their versions, tags and refs. Using one copies it in and spends nothing; a project with an older copy is offered the update. Animation clips get a library of their own: Mixamo FBX files and pack zips, or a rigged model's clips, kept once and skeleton only.

**Built for agents.** A local MCP server lets Claude Code or Codex drive the whole flow, with the same rules as the app (see [Agents (MCP)](#agents-mcp)).

## Real projects

Numbers from scenes I made with Maquette, raw source models against what the page downloads.

| Scene                                      | Source models | Shipped | Smaller by |
| ------------------------------------------ | ------------- | ------- | ---------- |
| A listening corner for a website, 10 props | 27.4 MB       | 0.89 MB | about 31x  |
| An island scene                            | 45 MB         | 6.3 MB  | about 7x   |

One prop from the listening corner: an armchair went from 3.56 MB and 19k triangles to 121 KB and 4k triangles.

The same room before and after the ship, from the site's own camera stops:

<table>
  <tr>
    <td><img src="assets/showcase/case-room.webp" alt="The room at the first camera stop before (left) and after (right) the ship: desk, window, shelf and a seated mascot look the same" /></td>
  </tr>
  <tr>
    <td align="center"><sub>The first camera stop, before (left) and after (right). The desk, window and shelf models went through the pipeline again and look the same.</sub></td>
  </tr>
  <tr>
    <td><img src="assets/showcase/case-radio.webp" alt="The radio up close before (left) and after (right) the ship, with the same grille, dial and knobs" /></td>
  </tr>
  <tr>
    <td align="center"><sub>The radio up close: 130 KB before, 135 KB after, the same look.</sub></td>
  </tr>
</table>

<!-- More before and after numbers and pictures go here. -->

## Agents (MCP)

Settings > Agents runs a Model Context Protocol server inside the app, so Claude Code and Codex can drive Maquette. It is off by default, listens on `127.0.0.1` only (port 4791, configurable) at `/mcp` over Streamable HTTP, and wants `Authorization: Bearer <token>`. The token is made the first time the server starts and kept in the app state on this machine, never in the workspace. New token replaces it.

Copy config gives a working setup:

```sh
claude mcp add --transport http maquette http://127.0.0.1:4791/mcp --header "Authorization: Bearer <token>"
```

```toml
# ~/.codex/config.toml
[mcp_servers.maquette]
url = "http://127.0.0.1:4791/mcp"
http_headers = { Authorization = "Bearer <token>" }
tool_timeout_sec = 150
```

The [MCP tools page](https://shironex.github.io/maquette/docs/agents/mcp-tools/) documents every tool with its arguments.

<details>
<summary><strong>Every tool, in one list</strong></summary>

Tools: `list_projects` (each project's id, name, template, target, site or game repo, created date, asset count, picture, what runs in it and which one is open in the app), `get_project`, `list_assets`, `get_pipeline`, `set_compression_presets` (per model texture size, data map size, codec and meshopt; only the given fields change, locked rows refuse, the Pipeline view updates live and nothing is spent), `get_material_tweaks` (`{ project, asset }`: the asset's material names, its tweaks, and the codec per map group with the raw and shipped sizes), `set_material_tweaks` (`{ project, asset, reset?, all?, materials?, codecs? }`; `all` and each `materials` entry take `tint`, `roughness: { mode: "override" | "multiply", value }`, `metalness` and `emissiveStrength`, a field left out keeps its value and null clears it, a material name the model does not have is refused with the list, and `codecs` takes `color`, `normal` and `other` as `"etc1s"`, `"uastc"` or null; everything is checked before anything is written, the tweaks land on `materials.json` as it is on disk, and when they cannot be written the codecs go back; the app updates live), `get_bake_settings` (`{ project }`, read only: the lightmap size, samples and denoiser, whether the project's `bake_light.py` can denoise, how it compares with the shipped starter, and the assets that skip the bake), `set_bake_settings` (`{ project, size?, samples?, denoise? }`: only the given fields change, a size or sample count out of range is refused, denoise on an older script is refused until it is updated, and the Pipeline view updates live), `get_starters` (`{ project }`, read only: each starter script in `tools/` as current, outdated, edited or missing), `update_starters` (`{ project, scripts?, replace? }`: the starter banner's Update for the named project whatever the app has open; without `scripts` every outdated or missing one, edited ones only with `replace` and kept as `<name>.bak`; a path that is no starter of the template is refused and nothing is written; the Pipeline view updates live), `get_run` (status and log tail), `list_inbox`, `get_inbox_checks`, `run_pipeline` (optionally from a step, optionally ignoring the cache, and with `buildOnly` stopping before Ship), `cancel_run`, `accept_inbox_model` (defaults to the asset the model was sent for, or the one a download was attached to), `render_thumbnail`, `start_concepts`, `get_concepts` (a direction's images, stars, picks and job states, with the failed jobs and why), `retry_concept_jobs` (`{ project, jobs? }`: failed or cancelled Lab jobs run again with their own prompt, refs and file, every failed one without `jobs`; queued, running or done ones are refused and nothing runs), `send_to_3d`, `send_batch_to_3d` (several assets under one approval; both answer with the weak ref warnings, which never stop a send), `redo_inbox_model` (`{ id, provider?, variant?, images?, overrides?, prompt? }`: an inbox model's asset sent again, prefilled from its send, under the same approval; the earlier model stays), `redo_inbox_models` (`{ ids, provider?, overrides? }`, one batch from one project), `get_credits` (`{ project?, limit?, balance? }`: each provider's balance, the week, month and redo totals, and the latest spends), `make_object_refs`, `get_ref_views` and `set_ref_views` (`{ project, asset, refs: [{ file, view }] }`: an asset's refs tagged `front`, `left`, `back`, `right` or `other`, at most one per view; sends follow the views and leave other refs home), `make_other_views` (`{ project, asset, views?, count?, useStyle? }`: Codex draws the empty views from the front) and `clean_ref_background` (`{ project, asset, file, background?, pad?, extend? }`), both Lab jobs like `start_concepts`, `use_ref_candidate` (puts a drawn view in its slot, or a clean copy in its ref's place with the original kept in `refs/originals`), `star_concepts` (`{ project, images: [{ file? | direction?, index?, starred }] }`: the Lab's Star, nothing goes to the Board), `pick_concepts` (`{ project, images: [{ file? | direction?, index?, star?, name?, kind? }] }`, 1 to 24 images: the Lab's Send picks to Refs one image at a time, so each image is copied into `refs/` and becomes or joins an asset at Refs, named after its object for object refs and after its direction otherwise unless `name` says; `kind` (`prop`, `character` or `scene`, never `scene` for object refs) applies to a new asset; an image picked before keeps its ref and answers with the same asset; every image is checked to be a concept of the project inside its folder before anything is written; it answers with each image's `refFile` and asset and each asset once with its `refs`, which `send_to_3d` then takes by default; nothing is spent and the Lab and the Board update live), `get_provider_job`, `cancel_provider_jobs` (`{ ids }`, 1 to 50 job ids: each is stopped on its own and answers with its new status and message or why it could not be stopped; a by hand send stops waiting, a Tripo or Meshy send not yet started is cancelled, and a send waiting for approval is refused because a person denies it in the app), `list_handoffs` (`{ project? }`, read only: every By hand send waiting for a model with its expected export name, handoff folder and estimated subscription credits, and what the Downloads watch matched, with the downloads that need a look as `needsLook` and the ones it could not place as `unmatched`), `attach_download` (`{ arrival, project, asset }`: sends one of them to the inbox for that asset, for its newest waiting By hand send or aimed at it, and answers with the `inboxFile` that `accept_inbox_model` then takes; nothing is attached without this call) and `dismiss_download` (`{ arrival }`: leaves it alone for good), `get_real_size` and `list_fixups` (`{ project, asset }`: the size, the fix-ups in order, what the newest model measures before and after them, the scale, the suggestion, the defaults and the last preview), `set_real_size` (`{ project, asset, size: { axis, value, unit } | null }`), `add_fixups` (`{ project, asset, fixups: [{ op, triangles?, maxSize?, minShare?, byMaterial?, by?, mergeUnder? }] }`, `split-parts` taking `by` (`loose` or `material`) and `mergeUnder`, a kind already there is replaced and a number left out starts from the workspace budgets), `remove_fixups` (`{ project, asset, ops?, all? }`) and `preview_fixups` (`{ project, asset }`, runs them headless on the newest version and returns the numbers before and after), `set_skip_bake` (`{ project, assets, skip }`: one or more assets by id or name leave the bake or join it again, all or nothing; `get_real_size` shows `skipBake`), `set_pad_edges` (`{ project, asset, pad }`: true or false pads the asset's texture edges or leaves them, null forgets the choice; `get_real_size` shows `padEdges`), `get_parts` (`{ project, asset }`: the parts as the newest model ships them, after running its split headless on a copy), `set_parts` (`{ project, asset, rename?, merge?, unmerge?, hide?, show? }`, parts named by id or shipped name) `compare_versions` (`{ project, asset, left: { version, fixed? }, right }`, read only: two sides part by part), `split_with_provider` (`{ project, asset, provider, version?, item?, model?, granularity?, splitByConnectivity?, complete?, mode?, parts? }`: a split on Tripo or Meshy that waits for approval like a send and lands in the inbox; a Meshy split's texture bakes back once accepted) and `assemble_prop` (`{ project, name, target?, parts }`: part models joined headless into one prop, a node per part, as the next version of an asset); each change is checked whole before anything is written, lands on `prepare.json` as it is on disk, and the app updates live, `get_layout` (the layout, every model with its bounds and size, the first view, the object states, the paths with their energize tracks and the clips kept off the site), `set_layout` (changes only what it names: `models` as `{ asset, position?, rotation?, scale?, remove? }`, `stops` as `{ id?, name?, position?, target?, fov?, models?, remove? }`, with no `id` adding a stop, plus `stopOrder` and `desk`; `states` as `{ asset, states, transitions?, triggers?, remove? }` sets an asset's object states whole, states and stops named by id or name, and `clips` as `{ asset, clip, ship }` keeps a clip off the site or ships it again; `paths` as `{ id?, name?, curve?, points?, start?, end?, width?, color?, glow?, remove? }` adds a path (no `id`, or a new one, with a name and points), changes one or removes it, `start` and `end` anchoring an end to an asset or letting it go with null, and `energize` as `{ path, easing?, keys, remove? }` sets a path's energize track whole; removing a stop drops the triggers and keys waiting for it; the whole result is checked before anything is written and the app's Layout and Motion views update live), `get_motion` (`{ project }`, read only: every model's rig with its bone map, its clips with their bytes in the source and as shipped, root travel and looping, its parts, the states, the clips kept off and the scene budget with the motion share in it and in the first view) `copy_asset` (`{ project, asset, to }`, the same copy as Copy to project...; a built asset arrives as a plain one), `remove_assets` (`{ project, assets }`, 1 to 50 ids or names: the same remove as Remove from project..., each asset answering `removed` with its trash id and path, `not-found` or `refused` with why, so a repeated call is safe), `list_removed_assets` (`{ project }`, read only: the project's removed assets in the workspace trash, newest first) and `restore_asset` (`{ project, trash }`: puts one back with its files, place and settings; refused while its name is on the Board again), `add_built_asset` (`{ project, script, helpers?, name?, kind?, root?, replace?, build? }`: the Board's Built asset, with absolute paths or paths relative to the project folder; it builds at once unless `build` is false and answers with the build), `rebuild_built_asset` (`{ project, asset?, all?, force? }`: one asset by id or name, or every new or stale one with `all`; files that match the last good build answer `up-to-date` unless `force`) and `list_built_assets` (`{ project }`, read only: each built asset's files, state and last build with its exported object and material names), `get_page_preview` (`{ project }`, read only: the site dev server's state, address, command and where it came from, who started it, why it failed and its last log lines, and `busyWith` when the one server serves another project), `start_page_preview` (`{ project, waitSeconds? }`: starts it or reuses the running one, waits up to `waitSeconds`, 30 by default, for it to answer) and `stop_page_preview` (`{ project }`: stops it whoever started it; a server Maquette did not start is only forgotten), `get_game` (`{ project }`, read only: a game project's settings and each asset's file and budget check), `scan_game_models` (`{ project }`, read only: every model in the model folder), `import_game_models` (`{ project, files }`: each file an asset with the repo file as version 1; nothing in the game repo changes), `set_game_settings` (`{ project, format?, modelsDir?, textures?, textureCap?, bake?, budgets? }`, refused whole when a value is out of range) and `get_ship_plan` (`{ project }`, read only: what Ship would write into the game, or for a website each file written or held back and why). `get_ship_settings` (`{ project }`, read only: a website's Ship style, texture format and quality, each asset's choice and file name, the site folders, the plan and the size report), `set_ship_settings` (`{ project, style?, textures?, quality?, assets?, models?, posters?, data? }`: only the given fields change, the folders go into the site's `maquette.site.json` keeping its other keys, and everything is checked before anything is written) and `get_size_report` (`{ project }`, read only: each model's source bytes and triangles against what ships, with totals, CSV and Markdown). `list_animations` (`{ query?, pack?, tag?, rig? }`, read only: the animation library's clips with pack, tags, source rig, frames, seconds, root motion and loop), `import_animations` (`{ paths, singles?, keepDuplicates?, keepFiles?, dryRun? }`: Mixamo FBX files or pack zips inside the workspace, read headless, skeleton only), `save_model_clips` (`{ project?, asset?, inbox?, clips?, pack?, tags?, dryRun? }`: a rigged model's clips as a pack, never clips that came from the library), `use_animations` (`{ project, asset, clips: [{ clip, name?, start?, end?, loops?, rootMotion?, speed? }], map?, saveMap?, dryRun? }`: a headless bake onto the model's newest version, reviewed per clip, failed clips left out and the rest added as its next version), `get_retarget_review` (`{ project, asset }`, read only: the last review and the clips baked in), `list_library` (`{ query?, tag?, project? }`, read only: the library's models with versions, tags and the projects that use them, and with `project` that project's assets with an update waiting), `add_to_library` (`{ project, asset, name?, tags? }`: the Board's Add to library), `use_from_library` (`{ entry, project, version? }`: a copy as a new asset at v1; a website project refuses an `.obj`) and `update_from_library` (`{ project, asset, dryRun? }`: the library's newer version as the asset's next version, or with `dryRun` the plan and no copy). The library tools spend nothing, need no approval and the app updates live. Built assets spend nothing and need no approval; a full `run_pipeline` rebuilds the stale ones first and is refused while one fails. The agent rules hold by construction: Blender only runs headless, no tool opens its window, and there is no screenshot or screen capture tool. A game project's run always stops before its Ship step for approval in the app. With Ask before shipping on, a run started over MCP stops before its Ship step and the app shows an Approve or Deny prompt; until then `run_pipeline` and `get_run` answer `waiting for approval`. A run whose Ship step would come from the cache does not ask. `send_to_3d` to Tripo or Meshy always waits for Approve in the app before any credit is spent, whatever that setting says, and Deny cancels the job; `send_batch_to_3d` shows one prompt that lists every asset with its settings and the total, and Approve or Deny covers them all; By hand spends nothing, so it writes the handoff folder straight away. Settings overrides are checked by name and an unknown one is refused with the valid list. Tools that work on a project take its id as `project`; none of them changes the project open in the app. Every call shows in the activity list under Settings > Agents.

</details>

## Requirements

Maquette runs on macOS and Windows. It calls tools you already have and checks for them on first run (`--version` on each; a tool that is not on your PATH can be located by hand).

| Tool                 | What Maquette uses it for                                                 | Needed       |
| -------------------- | ------------------------------------------------------------------------- | ------------ |
| Blender              | Every build, bake, export, poster and thumbnail, always headless          | Yes          |
| Codex CLI            | The Lab's concept images                                                  | For the Lab  |
| ffmpeg               | The Lab's contact sheet                                                   | For the Lab  |
| KTX-Software `toktx` | KTX2 textures in Compress; without it textures stay PNG or JPEG           | Recommended  |
| gltf-transform       | The Compress step (a global install, or through `npx` when Node is there) | For websites |
| Node                 | The Ship step (`tools/ship.mjs`), and a game's export and ship scripts    | To ship      |

Sending to Tripo or Meshy over their API needs that provider's API key, saved in Settings > 3D providers; By hand needs no key.

## Install

Download the latest build from the [releases page](https://github.com/Shironex/maquette/releases) (v0.1.0 is the first one): `Maquette_0.1.0_aarch64.dmg` for macOS on Apple silicon, or `Maquette_0.1.0_x64-setup.exe` for Windows x64. The builds are not signed by Apple or Microsoft yet, so the first launch asks once: on macOS open System Settings > Privacy & Security and choose Open Anyway, on Windows choose More info > Run anyway in SmartScreen. Updates through the app do not ask again. The [install guide](https://shironex.github.io/maquette/docs/start/install/) has the details.

## Develop

To run Maquette from source you need Node 22.12 or newer, pnpm 10 and the Rust toolchain pinned in `rust-toolchain.toml` (1.94.1), plus the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS.

```sh
pnpm install
pnpm dev        # the desktop app with hot reload
pnpm dev:web    # the UI alone in a browser, in mock mode
pnpm gate       # lint, types, tests, the docs site build, formatting, clippy and cargo test
```

`pnpm showcase` regenerates the images in this README from mock mode with [@noctcore/showcase-kit](https://github.com/noctcore/showcase-kit) (headless Chromium, a frozen clock, no network); `showcase.config.mjs` lists the shots.

<details>
<summary><strong>Mock mode</strong></summary>

Outside Tauri, a dev build answers every command from an in-memory backend (`apps/web/src/ipc/mock`), so the whole UI runs in a plain browser at `http://localhost:15190`. Pick a scenario with a query parameter:

- `?scenario=first-run` (default): no workspace yet, toktx missing
- `?scenario=configured`: a workspace with Frostveil open, its last run failed at Capture posters
- `?scenario=running`: like configured, with run 25 streaming and held at Export GLB
- `?scenario=idle`: like configured, the last run passed
- `?scenario=no-blender`: Blender missing, so setup stops at the toolchain step
- `?delay=700`: how long a toolchain check takes, in milliseconds
- `?pace=260`: milliseconds between streamed log lines in a mock run
- `?lab=running`: two concept images of the frozen lake arena direction mid job; `?conceptMs=4000` sets how long a fake image takes
- `?agents=active`: the MCP server on, with an agent's activity, a run waiting at Ship and a Send to 3D waiting for approval
- `?agents=batch`: like `active`, with three sends to Tripo waiting under one approval
- `?handoff=batch`: adds the veil gate props concept, the six object refs made from it (an objects direction) and a By hand batch of them in mixed states; Check Downloads finds two matches and a stranger
- `?style=board`: Frostveil has a style board at revision 3 (three images, a style line and four colours); Frozen lake arena carried it, Veil gate kit carried revision 2 and Elsheria sheets was made without it
- `?lab=refs`: the guardian statues' refs are tagged (a front that crops its object, a busy crop and a small detail as other), for the refs sheet, Make the other views, Clean background and the ref warnings
- `?arrival=named`: a moment after opening, an export named for the first waiting By hand send lands in the fake Downloads and is copied in; `?arrival=unmatched` drops `model (3).glb`, which asks "Which asset is this?"; `?arrival=look` drops a Tripo batch export whose name is only close to the Lantern post send (it needs a look) and two files that fit no send
- `?inbox=redo`: the veil gate (a Meshy send with a texture prompt) and Elsheria (a rigged Tripo send) in the inbox are linked to their sends for Redo, and a Meshy key is saved too
- `?built=demo`: two built assets on Frostveil at Blender, a current Laptop and a Newtons cradle whose script changed; Rebuild and Built asset work on them
- `?motion=demo`: Frostveil gains a clay figure on a 20 bone rig with three clips, a switch cabinet with three states and a turntable with one clip; `?motion=rest` has the figure without clips, `?motion=over` a scene budget the clips push it over, and `?motion=still` opens Frost cathedral, where nothing moves yet
- `?timeline=demo`: the `?motion=demo` models, with the switch cabinet opening its door at Veil gate, turning its breakers on from 55 to 70 percent and spreading apart from Keystone to Pullback; `?timeline=empty` takes Frostveil's camera stops away
- `?page=running`: the Page view's dev server starts as usual and the site holds the last ship, so Compare has a current ship; `?page=starting` keeps the server starting, `?page=failed` fails it as a site without its packages does, with its log, `?page=no-site` takes Frostveil's site repo away, `?page=stops-10` gives the page ten stops, and `?page=shim` takes the page's own bridge away and gives it the frame proxy's bridge shim. `?page=framed&site=<dev server>&frame=<frame proxy>` shows a real dev server behind a real frame proxy (the visual check starts both). The mock dev page is a small page of its own that speaks the page bridge
- `?game=crypt`: Crypt game, a CMake game with `.glb` models in `props/`, `enemies/` and `characters/`, is open with 15 imported models (3 over their folder's budget) and 3 made in Maquette, run 7 waits for approval before Ship, and the Inbox's ice crystals came back for Candle cluster, so they check against the props budget; `?game=import` opens it with nothing imported yet, `?game=maze` opens Maze game, an `.obj` game with 35 imported models
- `?projects=many`: eight projects instead of three (Frostveil, Frost cathedral, Elsheria showcase), so the switcher shows its search field

Configured scenarios also open with fake provider jobs: Veil gate generating at Tripo, Guardian statues waiting for a model by hand, Rune obelisk failed at Meshy for credits, and Elsheria accepted. Only the Tripo key is saved. The credits ledger holds those sends and a month of earlier ones. New sends move along on timers; nothing is sent anywhere.

The mock runner streams Blender style output and fails Capture posters once. The mock inbox lists Elsheria, two props and a broken download. The sample content is Frostveil, my own game: its previews load real meshopt and KTX2 models, and the Board and Inbox show real studio thumbnails, all from `apps/web/src/assets/mock` (that artwork is reserved and not covered by the code license, see [ARTWORK.md](ARTWORK.md)). For hand testing a real inbox, `cargo run -p maquette-core --example write_fixtures -- <dir>` writes small generated GLBs.

</details>

<details>
<summary><strong>End to end check</strong></summary>

`cargo run -p maquette-core --example new_project -- <workspace dir> <name> [--site <repo>]` creates a workspace (if needed) and a project with its pipeline and starter scripts, the way the app does. `cargo run -p maquette-core --example run_pipeline -- <project dir>` then runs a real project's pipeline from the terminal with the same runner the app uses, fills `{workspace}` and `{siteRepo}` like the app, and prints each step's state. A second run with nothing changed reports every step cached except Ship, which writes outside the project and always runs.

</details>

<details>
<summary><strong>Scripts</strong></summary>

| Script              | What it does                                                                                                                                                                                                                                                                                           |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `pnpm lint`         | ESLint over the workspace                                                                                                                                                                                                                                                                              |
| `pnpm lint:meta`    | Whole repo checks: 400 line cap per TypeScript module and 400 code lines per Rust module, no inline lint suppressions, no ESLint rule at `warn`                                                                                                                                                        |
| `pnpm typecheck`    | TypeScript in every package                                                                                                                                                                                                                                                                            |
| `pnpm test`         | Vitest for `apps/web` and `packages/ui`                                                                                                                                                                                                                                                                |
| `pnpm format:check` | Prettier check (`pnpm format` writes)                                                                                                                                                                                                                                                                  |
| `pnpm rust:fmt`     | `cargo fmt --check`                                                                                                                                                                                                                                                                                    |
| `pnpm rust:lint`    | `cargo clippy` with warnings as errors                                                                                                                                                                                                                                                                 |
| `pnpm rust:test`    | `cargo test` for the workspace, which also regenerates the IPC bindings                                                                                                                                                                                                                                |
| `pnpm gate`         | All of the above, in order                                                                                                                                                                                                                                                                             |
| `pnpm build`        | Release build of the desktop app                                                                                                                                                                                                                                                                       |
| `pnpm visual`       | Screenshots of mock mode in headless Chromium (`PLAYWRIGHT` picks another local Playwright `index.mjs` and `OUT_DIR` the output; `ONLY=handoff` renders only the object refs and batch shots, `ONLY=built` only the built assets, `ONLY=style` only the style board, `PORT` picks the dev server port) |
| `pnpm a11y`         | axe over every mock view and its main dialogs in headless Chromium, WCAG 2.2 A and AA plus best practices; fails on a serious or critical violation (`ONLY=<text>` picks cases, `ALL=1` fails on any, `PORT` reuses a dev server). Not in the gate: it needs Chromium and about a minute               |
| `pnpm showcase`     | The README images in `assets/showcase` and the banner, from mock mode with showcase-kit (`pnpm showcase:hero` renders only the banner; `SHOWCASE_PORTFOLIO_DIR` adds portfolio images)                                                                                                                 |

Commits follow Conventional Commits with a required scope (`desktop`, `web`, `site`, `ui`, `core`, `workspace`, `deps`, `docs`, `scripts`).

</details>

## Documentation

The [docs](https://shironex.github.io/maquette/docs/) cover the app view by view, each pipeline step and the MCP tools. Good places to start:

- [Install](https://shironex.github.io/maquette/docs/start/install/) and [your first project](https://shironex.github.io/maquette/docs/start/first-project/)
- [Website projects](https://shironex.github.io/maquette/docs/start/website-projects/) and [game projects](https://shironex.github.io/maquette/docs/start/game-projects/)
- [The Board](https://shironex.github.io/maquette/docs/views/board/), [the Inspector](https://shironex.github.io/maquette/docs/views/inspector/) and [the pipeline steps](https://shironex.github.io/maquette/docs/pipeline/pipeline-json/)
- [Connect an agent](https://shironex.github.io/maquette/docs/agents/connect/) and [the MCP tools](https://shironex.github.io/maquette/docs/agents/mcp-tools/)

The reference below is the long form of the same, kept here for reading next to the code.

## Reference

<details>
<summary><strong>Projects</strong></summary>

A workspace holds any number of projects. The project switcher at the top of the rail (or Cmd+P on macOS, Ctrl+P on Windows and Linux) opens a menu of them with their picture, template, asset count and age; the open one is checked, and a search field shows once there are more than six. Arrow keys move, Enter opens, Escape closes. New project opens the onboarding project form in a dialog (where it ships, template, name, target site repo or game repo) and Import folder the import form. The open project is saved as `currentProject` in `maquette.json`.

Switching starts every view of the new project clean: Board, Lab, Pipeline, Inspector, the 3D queue and the status bar. Runs, Lab images and provider jobs keep going in the project they belong to, and the menu says what is still running there. The Inbox is shared by the whole workspace: a model sent from another project names that project under Sent for, and Accept moves it into the project it was sent for. The rail collapses to icons (Collapse at its foot); the switcher and its menu work there too.

Board layout in the Board's header switches between the stage columns and a list: one dense row per asset with its picture, kind, stage, newest version, triangles, size (the shipped build once Compress ran, else the model file, marked source), status and last change. A heading sorts by its column (again flips it), the stage filter and the search narrow the rows, and the layout, sort, filter and search are remembered per project on this machine. Rows take the same boxes, Shift ranges, selection bar and menu as the cards; with a row focused, the arrow keys move, Shift and an arrow extends the selection, Space ticks the row and Enter opens its menu.

Copy to project... in a Board card's menu copies the asset into another project and leaves it where it is: its refs, every model version, thumbnails and finished sends go along, renamed to the new id. When the other project already has an asset by that name the copy is called "Name 2". A send still running stays with the original.

Remove from project... in the same menu, or Remove... with several cards selected, takes assets off the Board. Nothing is deleted: each asset's refs, model versions, thumbnails, handoff folders and settings move to `trash/<project>/<asset>-<time>/` in the workspace with a `removed.json` record, and the Removed button on the Board (or Undo on the notice) puts it back with its place in the scene. Sends waiting for a model are stopped first; an asset whose Tripo or Meshy model is being made right now, or that is building, is refused, and so is any remove while a run is going.

In the Lab, an earlier direction's menu has Move to project..., which moves its folder (concepts, picks, stars and brief) into another project. The refs it sent and its Board card go along when the card has no model yet; a card with models, or one with a send under way, stays where it is.

</details>

<details>
<summary><strong>Game projects</strong></summary>

A project ships to a website (the default) or to a game. New project has Website and Game under Ships to; a game project takes a **Game repo** instead of a site repo and stores its settings under `game` in `project.json` (`target: "game"`; the repo itself stays `siteRepo`). Existing projects read as websites.

Picking the game repo checks it (`maquette_core::game::detect`): git and its branch, `CMakeLists.txt` and the executables it builds, the model folder (the first of `assets/models`, `assets/meshes`, `models`, `res/models` and `data/models`, with model files up to four folders deep), the format most of its files are in (`.glb` or `.obj`, preselected and marked detected in the format picker), the loader named in the build files or a vendored folder (cgltf, tinygltf, fastgltf, tinyobjloader, Assimp) and whether the engine can decode KTX2 or meshopt. A website repo with a `CMakeLists.txt` gets a hint to pick Game. There are no game templates: after Create, Import models scans the model folder and lists every model with its folder, format, triangles, textures, size and budget check. Each picked model becomes an asset whose version 1 is the repo file and joins the ship record; the ones left out stay hand made and Ship never touches them. Import models on the Board scans again later.

Budgets are per folder (`game.budgets`, a triangle count and a largest texture edge for `props`, `enemies`, the top level and so on, with the nearest folder above as the fallback); there is no scene total. A model over its budget is flagged on its Board card and list row, counted in the top bar's Model budget, and listed in the Inspector's Budget tab, which edits each folder's numbers and adds a Decimate or Shrink textures fix-up to every model over (Build models applies it on a copy).

The game pipeline (`pipeline/template/game.rs`) is Build models (`build_models.py`), Bake light only when `game.bake` is on (off by default), Export (`export_game.py`: one plain file per model at its path in the game, `.glb` with PNG or JPG textures, or `.obj` with its `.mtl` and texture files, textures capped at `game.export.textureCap`, rigs and clips kept), Game export (`game_export.mjs`: lays the files out in `web/game/` as the model folder keeps them, copies untouched imports byte for byte, and refuses KTX2, meshopt, Draco or a texture over the cap in an exported model) and Ship (`ship_game.mjs`). There is no Capture posters step and no camera stops, and the stage after Blender reads Export. The Pipeline view's Game export panel sets the format, the texture format (PNG, JPG or Keep), the size cap and Bake light; LOD levels show as not built yet.

Ship to a game always waits for approval in the app, whoever started the run, and the Pipeline view opens the Ship diff: each file as new, replaced (in the ship record and different), unchanged (same bytes) or hand made (not in the record, left alone even when an export would land on it), where it writes, the record size after the run and the checks. Ship writes into the model folder keeping subfolders, only replaces files the record lists (`web/ship-record.json`), never removes a file and never commits. Page and Timeline are hidden for games (Motion stays), the Inspector shows Compare, Budget and Parts, and Game preview is listed as planned.

</details>

<details>
<summary><strong>Pipeline runner</strong></summary>

Each project keeps its steps in `pipeline.json` (new projects start from the Floating diorama template: Build, Bake light, Export GLB, Compress, Capture posters, Ship). A step has an `id`, a `kind` (`blender`, `shell`, `gltf-transform`, `toktx`, `ffmpeg`), a script or command, an args array, `inputs` and `outputs` globs and `depends_on`.

- Programs are spawned directly with an args array, never through a shell, on the same PATH the toolchain check used. On Windows they start with `CREATE_NO_WINDOW` inside a job object, so Cancel ends the whole process tree.
- Blender always runs in the background: a `blender` step without `-b` is refused. The only time Maquette opens Blender's window is when you click Open in Blender.
- A step is skipped when its input files, script, args and tool version hash to the same key as the last good run (kept under `cache/`). Run from here starts at a step; Ignore the cache reruns everything.
- One run at a time per project; more runs queue. Runs and their logs (last 5) live in the project's cache folder.
- The Compress step reads the per model presets (texture size, ETC1S or UASTC, meshopt level). A row can pick the codec per map group (`mapCodecs`: `color` for base color and emissive, `normal`, `other` for metal, roughness, occlusion and the rest); a group left out follows the row's codec, except normal maps, which default to UASTC (a locked row without `mapCodecs` keeps its codec for normal maps too, so its output does not change). Each group that differs gets its own texture pass (`--slots` with an extglob such as `@(baseColorTexture|emissiveTexture)`), and a model without normal maps skips that pass. UASTC normal maps cost bytes: on a 1K test prop the shipped file went from about 360 KB (all ETC1S) to 1.1 to 1.4 MB, so a prop that does not need them can set its normal group to ETC1S.
- Material tweaks (below) are written into a copy of each model before its texture passes, under `cache/`; the raw export never changes. The tweaks are part of the step's cache key, so changing them reruns Compress. A locked row keeps its values and says why. A scene project gets the locked character row only once it has a character asset; a Character showcase starts with it. Rows read as the asset a model was exported from (`clay_v1.glb` is "clay").
- A running node shows a progress bar when its tool reports how far it is: a script's `progress N/M` or `progress P%` line (with an optional `[tag] ` prefix), Cycles' `Sample N/M` inside the current item, or one finished call of several (Compress runs one chain per model). Otherwise the bar is indeterminate.

#### Starter scripts

A new project gets small, commented scripts in its `tools/` folder when its pipeline is first written, so it runs end to end on day one. They read their paths from args and are the project's to edit; Maquette never writes over a file that exists on its own.

`tools/.starter.json` records the hash of every starter Maquette wrote, so a later version can tell an untouched copy of an older starter from one you edited (scripts from before that record are matched against the known older starters). When newer starters ship, the Pipeline view says New starter scripts available and Update brings the untouched ones up to date in one go. An edited script is never overwritten silently: it shows as edited, with Replace with the new starter (your file is kept as `<name>.py.bak`) or Keep mine.

- `build_scene.py`: opens `blender/scene.blend` when there is one (else an empty scene with a sun), imports each `models/*.glb` into its own collection under "Models", gives each model its fix-ups and real size from `prepare.json` (below) through `fixups.py`, places them from the scene layout (`layout.json`, below) and saves `blender/scene_built.blend`. Models without a placement that overlap are laid out side by side. The layout's desk becomes a slab under the models, so they shadow it in the bake, unless a model already is the floor there (a room shell whose floor reaches the desk top over half the slab), because a slab on that floor would hide it from the light. The world is a mid grey (0.18), so the bake has a soft fill. Models set to skip the bake are tagged for the bake and the export. Its camera stops become cameras in a "Stops" collection unless the base blend has stop cameras of its own.
- `fixups.py`: the fix-ups and the real size scale, imported by `build_scene.py` and also run on its own (`--input --plan --output --stats`) for the previews in the Inbox, the Board and the Inspector, which write a fixed copy and its numbers under the workspace `cache/fixups/`.
- `bake_light.py`: gives every mesh outside "Studio" a second UV named `lightmap` (Smart UV Project when it is missing) and bakes Cycles diffuse light (direct and indirect, or the full look with `--type combined`) into one `blender/bake/<object>_light.png` per object, on the CPU, then fills the gaps between UV islands and denoises each map with OpenImageDenoise (`--no-denoise` turns that off). The size and samples come from the step's args (`--size 1024 --samples 32` for new projects, see Baked light below). Meshes of models set to skip the bake, and meshes whose every material glows on its own (an emission strength above 0 with a colour or an image, such as a sky picture), get no lightmap and no second UV; the log says how many and why. The whole scene bakes together, so every model shadows the others and the desk. Each map stores light divided by a scale of at least 1; `bake.json` lists every object's file, size and scale. It prints `progress N/M` per object and saves `blender/scene_baked.blend`. A scene without meshes writes `"baked": false` and passes.
- `export_web.py`: one GLB per model collection (and per other top level object) into `web/raw/`, named in snake case, uncompressed for the Compress step. Objects and materials Blender renamed on import (a second `palette` becomes `palette.001`) export under the names their model file gives them; Build scene tags them and the export swaps the names back for each file. A baked object keeps both UV sets (`lightmap` is `TEXCOORD_1`), carries its lightmap as the emissive texture on that UV and has the material extras `{"lightmap": <scale>, "lightmapUv": 1}`, so a page can show albedo times lightmap times scale in an unlit material. A material that glows on its own keeps its emission as it is, and a mesh that was not baked ships without the `lightmap` UV.
- `render_stops.py`: one poster per camera stop, a camera with the `maquette_stop` custom property (its number sets the order) or a name starting with `stop_`. A scene without stops gets one framed three quarter poster of the whole scene instead (`overview.png`, from a temporary camera fitted to the scene bounds that is never saved into the blend), and the log says so. It always writes `web/posters/stops.json`, with each stop's name from the layout as its `label`, and removes the posters the previous one listed.
- `ship.mjs`: copies `web/dist/*.glb` and the posters into the project's site repo, and writes the site's scene data when the site asks for it. Without a site repo it says so and passes. A site says where things go in `maquette.site.json` at its root (`{ "version": 1, "models": "public/scene/models", "posters": null, "data": "src/scene/shipped.json" }`, folders relative to the site and inside it, `posters: null` copies none); a site without the file gets `public/models` and `public/posters` and no data, as before. The data file lists each asset's shipped model (from the index Maquette writes before every run, `{shipIndex}`), marked `inPlace` because the export keeps every model where the layout put it, plus the project's `layout.json` and `materials.json` as they are. It is only written from a layout with placed models and camera stops: Ship stops before an empty layout could wipe the site's scene, and checks every folder before it copies anything. With a data file, Ship copies only the models the data lists, so a leftover in `web/dist` never reaches the site (Compress also removes every model in `web/dist` that no current model makes, logging each one, and Export GLB clears `web/raw` before it writes). Ship keeps `web/ship-record.json` with the model files it shipped into each site folder; a file it shipped last time and no longer ships is removed from the site and logged, and every other file in the site is left alone.

Projects made before the site data get the new `ship.mjs` through the Update banner when their copy is untouched, and an untouched Ship step gets the new args (`--index {shipIndex} --layout {project}/layout.json --materials {project}/materials.json`) once the script reads them.

**Preview on site** in the Pipeline view's header starts the site's own dev server in the site folder and opens it in the browser once it answers. It is the same one server the Page view runs (below), with the same command: the dev command under Settings > Project, else the `preview` of `maquette.site.json` (`{ "command": ["pnpm", "dev"], "url": "http://localhost:15292/" }`), else the `dev`, `start` or `serve` script of the site's `package.json` run by the manager of the nearest lockfile on a free port. A server that already answers at that address is reused and never stopped, the app stops the one it started when it quits, and only addresses on this machine open. Nothing is built or deployed; deploying stays your step.

Rigged models keep their armature, skin and clips through Build scene and Export GLB: the export writes every action as one glTF animation under its own name, sampled at the scene rate, and logs how many each file holds. Build scene leaves out the clips the Motion view keeps off the site (`clips` in `layout.json`) and logs them; projects whose `build_scene.py` predates that ship every clip until the Update banner brings the untouched copy up to date, and the Motion view says so.

Projects made before the layout editor get these `build_scene.py` and `render_stops.py` through the same Update banner when their copies are untouched; the Build scene step then reads `layout.json` too, so saving a layout reruns it. In the same way, an untouched `build_scene.py` updates to the one that reads `prepare.json`, `fixups.py` arrives next to it, and both files become inputs of the Build scene step, so a changed size or fix-up reruns it.

#### Baked light

The Pipeline view's Baked light panel sets the project's lightmap size (256 to 4096 px), Cycles samples (1 to 4096) and the denoiser, and says roughly how the bake time compares with the default; agents use `get_bake_settings` and `set_bake_settings`. They are the Bake light step's `--size`, `--samples` and `--denoise` args in `pipeline.json`, so editing the args by hand is the same thing, and a change reruns the bake. New projects bake at 1024 px with 32 samples, denoised. Measured on a study room (Apple M3, CPU): 512 px at 16 samples without the denoiser took 20 s for 33 meshes and left blotches on flat surfaces; with its 9 scans skipping the bake, 1024 px at 32 samples took 143 s for 21 meshes and is clean, and 64 samples took about 300 s and looks the same once denoised. Time grows with the texels and the samples, so 2048 px costs about four times 1024 px. The denoiser adds about half a second per map. An older `bake_light.py` cannot denoise, so the panel keeps the switch off until the Update banner (or `update_starters`) brings the new starter in. The banner names the project it updates, since it acts on the project open in the app, and the toast after it names the project and the scripts.

An asset can skip the bake (Skip the bake in the Inbox, a card's Real size and fix-ups... or the Inspector, or `set_skip_bake`). It then keeps its own material lighting, still shadows the baked models, and ships with no lightmap and no second UV set, which is smaller and right for provider scans: their many small UV islands bake into flat lit facets. A model accepted from the Inbox skips the bake unless you choose otherwise.

Projects made before these settings get the new `build_scene.py`, `bake_light.py`, `export_web.py` and `fixups.py` through the Update banner when their copies are untouched.

Character showcase projects get `build_character.py`, `export_character.py`, `render_stops.py` and `ship.mjs`. The build imports the newest `models/*.glb` (highest `_v<n>`, then newest file) with its rig and clips, on top of `blender/character.blend` when you add one, adds a key and fill light when the scene has none and a front camera stop `stop_01_front` when it has no stop, and saves `blender/character_built.blend`. Export GLB and Capture posters read that file, so the pipeline runs as soon as a character is in `models/`.

</details>

<details>
<summary><strong>Model inbox</strong></summary>

Maquette watches the workspace `inbox/` folder for `.glb` and `.gltf` files. Each file is read in Rust: triangles, meshes, materials, textures and their sizes, skins and joints, animations. Checks compare that with the budgets: triangles, texture size, a Mixamo rig for characters, PBR maps, separate objects merged into one mesh, an origin away from the bottom centre, several objects that could be one, and hair or tail vertices weighted to finger bones. In a game project, a model sent for an asset checks its triangles and textures against the budget of the folder that asset's model goes to ("Within the props budget, 12k tris"), the same budget the top bar counts against; a model nobody sent, and every model in a website project, uses Tris per model and the default texture size. A check that a fix-up answers offers it as a button (Decimate to 60k, Shrink textures to 2048 px, Remove loose parts, Split into parts, Origin to bottom centre, Merge objects). Accept to Blender moves the file into the project's `models/` folder as a new asset version.

Two checks are heuristics:

- **Merged objects:** a mesh whose welded triangles form several islands far apart (gaps over 10% of the mesh size, each island at least 1% of it) probably holds objects that were separate. Scenes are exempt.
- **Weighted to finger bones:** a vertex whose strongest joint is a finger bone, but which sits more than 15% of the model height away from that bone (or above the head joint), is counted. Draco or meshopt compressed geometry skips both checks.

The preview stands every model in the same studio: a graphite backdrop, a soft floor with a contact shadow, a warm key, a warm fill and a cool rim light, and a three quarter camera framed to the bounds. Shaded, wireframe over clay, a numbered UV checker, and the skeleton drawn in clay over a ghost of the mesh. It reads meshopt geometry and KTX2 textures (the Basis transcoder ships in `apps/web/public/basis`).

#### Real size and fix-ups

Each asset can have a real size and a list of fix-ups. Both are optional and are set in the Inbox before Accept (next to the measured bounds), and later from a card's (or list row's) Real size and fix-ups... or the Inspector's model details. A size is one side of the real object: height, width, depth or the longest side, in cm or m (0.1 cm to 1000 m). A small table of everyday objects suggests one from the name and kind (a mug 10 cm high, a laptop 32 cm wide, a desk lamp 45 cm high, a character 170 cm high); a suggestion only applies when you pick it.

The fix-ups are Apply transforms, Remove loose parts (parts up to a share of the model's diagonal, 2% by default), Merge objects (into one mesh, or one per material), Split into parts (by loose pieces or by material, bits under 1% of the model merged into a neighbour; see [Parts](https://shironex.github.io/maquette/docs/views/parts/)), Decimate (to a triangle target, the per model budget by default), Shrink textures (to a longest edge, the default texture size by default) and Origin to bottom centre (the model stands on the floor at the origin). Each kind appears once and they always run in that order. Rigged meshes keep their transforms and shape keyed meshes are not decimated.

The same places set Skip the bake (see Baked light above): the asset's model ships with its own material lighting instead of a lightmap.

They also set Pad texture edges (or `set_pad_edges`). Before Decimate and Shrink textures, Build scene fills the texture around each UV island with the island's own colours: a few exact steps of each gap pixel taking the mean of its painted neighbours, then a coarse to fine fill for the rest. Without it a shrunk or mipmapped texture blends the island borders with whatever the gaps hold, which shows as light or dark seams. A model accepted from the Inbox pads unless you choose otherwise, since provider scans cut their textures into many small islands; built assets and models added by hand do not. Previews skip it. On a 626k triangle scan it adds about 9 s to Build scene, and the shipped file comes out a few percent smaller, since the filled gaps compress better than noise.

Nothing touches the inbox file or an asset's versions. The Build scene step imports each model and applies its fix-ups in Blender (headless), then scales it uniformly so the chosen side measures the real size, and only then places it from the layout, so the layout's scale stays a tweak on top. The fix-ups replay on every new version of the asset. Preview the numbers runs the same script headless on a copy and shows triangles, objects, largest texture, size and file size before and after, with what each fix-up did; a plan that already ran answers at once. The Layout editor draws each model moved and scaled the way the build will place it, and its sizes read in real units.

They live in `prepare.json` in the project, keyed by asset id, with each asset's part names, joins and hidden parts under `parts`, saved like the material tweaks (laid over the file on disk; a file that cannot be read, or one from a newer Maquette, is refused and left alone). Accept saves the Inbox's choice for the asset; Inbox choices not accepted yet count as unsaved work when the app quits.

```json
{
  "version": 1,
  "models": {
    "desk-lamp": {
      "size": { "axis": "height", "value": 45, "unit": "cm" },
      "fixups": [{ "op": "decimate", "triangles": 5000 }, { "op": "origin-bottom" }],
      "skipBake": true,
      "padEdges": true
    }
  }
}
```

#### Thumbnails

Inbox models get a 256 px thumbnail when they arrive, and Board assets get one on Accept and for each new version. A headless Blender job (always `blender -b`, never the GUI) renders the model in the studio above, on the runner's own `thumbnails` lane, so renders queue and never block the UI. Renders are cached by content hash in `cache/thumbs/` and copied to `<project>/thumbs/<asset>_v<n>.png`. When Blender is missing, or cannot read the file (KTX2 textures), the UI draws the model once offscreen with three.js in the same studio and stores that instead. While a render runs, cards and rows show the asset kind's icon.

A compressed model is rendered from an uncompressed copy, because Blender reads neither KTX2 textures nor meshopt geometry. First match wins: the file itself when Blender reads it; else its uncompressed export (`../raw/<name>` next to a `dist` folder, then `raw/<name>`, then the project's `web/raw/<stem>.glb`); else the three.js snapshot above.

Render thumbnails by hand with `cargo run -p maquette-core --example render_thumbnail -- <thumbs dir> <model>...`.

</details>

<details>
<summary><strong>Built assets</strong></summary>

A built asset is a model a Blender script in the project makes, the fourth source next to Tripo, Meshy and By hand. **Built asset** on the Board (or `add_built_asset` over MCP) takes the script, the helpers it imports or reads (a shared `common.py`, a texture), a name (from the script's file name until you type one), a kind and, optionally, a root object. The script and helpers are copied flat into the project's `built/` folder; they are the project's to edit from then on, and a helper several assets share is kept once (a different file under a name already there is refused unless you tick Replace). The asset lands at Blender and builds at once.

- **The script:** it is imported as a module named after its file, with `built/` on `sys.path`, in Blender started headless with factory settings (`blender -b --factory-startup`). When it has a `build()` function, that is called and the object it returns is exported with its children. Otherwise the root object option names it, and without either the whole scene exports. Object names, parents, origins, custom properties and materials stay as the script made them, so names a page looks up (`lid`, `screen`, `ball_1`) and hinge pivots survive. Cameras and lights are not exported.
- **Where it runs:** in a scratch copy under the workspace cache (`cache/built/<project>/<asset>/`), on its own runner lane, one build at a time for the whole app, with a 10 minute limit. A script that writes next to itself (an `out/` or `renders/` folder) writes there, never into the project.
- **A version:** a good build becomes the asset's next version, `models/<id>_v<n>.glb`, with no Inbox, no credits and no approval. The Inbox checks still run on it and the ones that do not pass are kept on the build as warnings; Separate objects, Merged objects and PBR maps are skipped, since a script's separate pivots and plain materials are on purpose. A build that makes the newest version's exact bytes again adds no version. A thumbnail renders for each new version as for any other.
- **When it builds:** `built.json` in the project records each built asset's script, helpers, root and last build. A build key hashes the runner, the script, each helper by name and content (CRLF read as LF) and the root. A card whose files match its last good build is current; one whose files changed says Script changed and offers Rebuild; one that failed says Build failed with the script's error. Rebuild on the card always runs Blender. A full pipeline run (not Run from here) first rebuilds the new and stale built assets and does not start while one is missing a file or fails to build. The Blender version is not in the key: after a Blender update, Rebuild the ones you want rebuilt.
- **One script, several models:** each asset names its root object, and several assets can share a script. A script that builds every piece in one go and resets the scene between them (a `build_all.py`) is not one of these; add each piece's own script instead.

The real size, fix-ups, layout and material tweaks apply to a built model as to any other. Merge objects and Apply transforms flatten pivots, so leave them off for a model with moving parts.

Build one from the terminal with the real Blender: `cargo run -p maquette-core --example build_asset -- <workspace dir> <project name> <script.py> [--helper <file>]... [--name <name>] [--kind prop] [--root <object>] [--rebuild]`.

</details>

<details>
<summary><strong>Concept lab</strong></summary>

The Lab turns a brief into concept images with the Codex CLI. A brief is a direction name, a prompt, up to 8 reference images (dropped in, picked from the project's `refs/`, or chosen from a file dialog), an aspect ratio and 1 to 12 images. Each image is one Codex job:

```sh
codex exec --skip-git-repo-check -s workspace-write -i <ref> -i <ref> -- -
```

The prompt goes on standard input, starts with "Generate an image." and ends by asking Codex to copy the result out of `~/.codex/generated_images/` to an exact path, `concepts/<direction>/<direction>_NN.png` in the project. `-i` takes several values, so `--` ends the list before the `-` that reads the prompt. Jobs run in the project folder on their own lane, 4 at a time by default (Settings, `lab.concurrency`, up to 8), and each is stopped after `lab.timeoutMinutes` (10). A job succeeds when the file appears. If Codex saved the image but not the copy, the newest image from that Codex session is picked up instead; otherwise the job fails with the last error line of its log. Codex's own `hook: ...` lines are left out of the job's log tail and its error. When Codex answers 429 Too Many Requests (or says it was rate limited) the job says "Rate limited by Codex" and waits, shown in the queue as "Waiting, rate limited, retries at HH:MM", then runs again on its own after 3, 6 and 12 minutes; after that it fails and asks you to try again in a few minutes. When Codex says its usage limit is hit ("try again at 4:21 PM", "in 37 minutes"), the job waits until that time in local time plus a minute (tomorrow when the time has passed today), shown as "Waiting for the Codex usage limit until about HH:MM"; every queued job, and any started meanwhile, waits with it instead of running into the limit, and all run again on their own then. A usage limit with no time Maquette can read waits like a rate limit. Retry on a failed or cancelled job in the queue (object ref jobs too), or `retry_concept_jobs` over MCP, runs the same prompt, refs and target file again under the same job, without making a new direction; on a waiting job it runs at once, and Cancel stops the wait.

Results show per direction with Star and Pick and a lightbox (arrow keys, S, P, Escape). Contact sheet tiles every image with ffmpeg into `contact_sheet.png` next to them. Send picks to Refs copies the picked images into `refs/` and moves the asset with the direction's name to Refs on the Board, or adds it there. Agents do the same over MCP with `pick_concepts` (and Star with `star_concepts`). The brief and marks live in `concepts/<direction>/direction.json`.

Rename... in a Board card's menu changes only the name on the card: the asset's id, files and versions keep their names, so refs, inbox matches and pipeline outputs stay linked.

Make object refs (on a Lab image, or in the menu of a Refs card whose picture is a ref) turns one concept into a clean reference image per object, for a scene that needs many props. List the objects as chips (Enter, a comma, or a pasted list with one per line; up to 12, each name 60 characters at most, shown with a counter and never cut), click a chip to rename it or add notes for Codex (they steer the image and are not part of the asset name; `make_object_refs` takes them as `notes`), pick the aspect (1:1 by default) and whether they become props or characters. Each object is one Codex job on the same lane, with the concept attached as `-i`, and a built in prompt asks for that object alone: centered, the whole of it in frame, a front three-quarter view, a plain light grey background, even soft light, no cast shadow on the background, no text, and the concept's materials and style. The images land in a new direction, "<name> objects", each titled with its object. Another concept with the same name gets its own direction ("<name> objects 2", then 3), so neither loses its source or kind; the same concept again adds to its direction. Send picks to Refs there copies every pick to `refs/<object>.png` and makes one asset per pick, named after the object; Send to 3D does that and opens the batch panel for all of them.

The style board (the palette button under the brief's references) keeps a project's directions in one look: up to 4 images copied into `style/`, one style line and up to 6 palette colours, saved with a revision in `style.json` at the project root. Once it holds anything, every new direction and object ref adds the line and palette after the brief's prompt and attaches the board's images after the brief's own refs, so the brief leads. Use the project style turns it off for one brief, a direction's chip says which revision it carried (or Own style), and Retry keeps the revision a job started with. A concept's palette button adds it to the board. Agents read and change it with `get_style_board` and `set_style_board`, and `start_concepts` and `make_object_refs` take `useStyle: false`.

`cargo run -p maquette-core --example generate_concept -- <project dir> --direction "Stone lantern" --prompt "..." [--ref refs/a.png] [--count 1]` runs the same jobs from a terminal. Every image spends Codex usage.

</details>

<details>
<summary><strong>Send to 3D</strong></summary>

Send to 3D takes refs from a Board card (its menu) or the picked concepts of a Lab direction (the picks go to Refs first) to Tripo, Meshy or By hand. The panel starts from the Settings preset for the asset (the prop or the character variant, picked by asset kind), and every field can change for this send only. One image makes one model; with Multi-view on, up to four images go as front, left, back and right.

- **Tripo** (REST v3): upload, image to model or multiview to model, poll, download. A character can add the rig chain: rig check, rig, then retarget the chosen animations. Tripo has no cancel, so a sent task always finishes and lands in the Inbox.
- **Meshy** (REST v1): image to 3D or multi image to 3D (images go up as data URIs), poll, then rigging for characters. Cancel works while the task is still pending; once Meshy has started it answers 409 and the queue says so.
- **By hand:** no API call and no API credits. Maquette writes `projects/<p>/handoff/<asset>-<n>/` with the images named by view and a settings card (`settings.md` and `settings.json`) for the service's web app. The panel shows the same card with copy buttons, Open Tripo studio (or Open Meshy) in the system browser, Open folder, and Reveal image for each image, so it can be dragged into the browser. The asset waits on the Board until the model comes back: Add model... on the card or in the panel (a `.glb`, or the `.zip` a web app downloads, which must hold one `.glb`), drop either on the card, or save it into the inbox under its export name. Check Downloads looks in the Downloads folder (the OS known folder) for `.glb` and `.zip` files newer than the oldest waiting handoff and suggests a waiting send for each, by file name and time; nothing is taken until you press Use on a row. Every way copies the model into the inbox, leaves the original where it is, and runs the Inbox checks.

  The card also says about what the send costs on the site's subscription (one table in `providers/subscription.rs`: about 30 credits for Tripo textured image to 3D, 20 for Meshy mesh only), has Copy prompt when the send has a texture prompt, and names the export `maq_<asset>_v<n>_<provider>` (n is the send number). The handoff folder holds `send.json` with the asset, version, provider, prompt, estimated credits and that expected name. Maquette never opens or drives the provider's site beyond a plain link, and uses no cookies or private endpoints. The Downloads watch (Settings > 3D providers, on by default, with its own folder and an off switch) runs only while a By hand send waits: it notices new `.glb`, `.fbx` and `.zip` files, waits until their size stops changing, skips files it has already seen (by hash), and copies a file named for its send (the export name) into the inbox with a quiet notice. It never moves or deletes a file. A file whose name is only close to a waiting send's (a site's batch export such as `lantern post 3d model.glb`), or that came for the only send waiting, lands nowhere: it waits under Needs a look with its model's headless thumbnail next to the send's ref, the file name and why it matched, and Attach to the asset, Pick another asset or Not this one (left alone and never asked about again). A file it cannot place asks "Which asset is this?" and Attach to an asset opens a searchable list of the project's assets, waiting sends first, that moves on to the next file after each pick. Attaching sends the model to the inbox for that asset, as a file named for its send goes: for the asset's newest waiting By hand send, or aimed at the asset when none waits, so Accept goes there by default and closes its By hand sends still waiting. An `.fbx` is matched but left for a GLB export. A By hand send goes into `credits.jsonl` as a subscription spend (estimated, settled when its model is imported), shown under Subscription in Credits, apart from API spend, and in the CSV with a `source` column.

Several assets go in one batch: tick the boxes on the Board cards (Shift selects a range) and press Send to 3D in the selection bar, or Send to 3D in an objects direction. One provider and one copy of the presets cover the batch, each asset keeps its own kind (prop or character), and each gets its own job and model. A Tripo or Meshy batch is checked whole first and asks once, listing the task count and settings, before anything is spent. A By hand batch writes one handoff folder per asset and opens the By hand checklist: every handoff of the project grouped by batch, with its settings, folder, status, Add model... and Check Downloads. The By hand button on the Board opens it again.

Redo sends an inbox model's asset again from the send it came from: Redo... on the Inbox detail, or in a row's menu. The send panel opens prefilled with that send's provider, images, kind and settings; change what should differ (Meshy also takes a texture prompt of up to 800 characters, which goes to Meshy as `texture_prompt` and on a By hand Meshy card; Tripo makes the model from the images alone). The redo is the asset's next send and next version, and the earlier model stays in the inbox, so both can be compared and one accepted. Tick several inbox rows (they must come from sends of one project) and press Redo in the bar under the list for one batch, each asset prefilled from its own send. A paid redo asks exactly like any other send, and a By hand redo writes a new handoff folder.

The send panels, the batch confirm and an agent's approval prompt show what a send should cost, from the providers' published price lists (Tripo's H series table, Meshy's pricing page). Every paid send is kept in `credits.jsonl` at the workspace root, one JSON line per event (sent once the provider has a task, then succeeded, failed, refunded when the provider called the model task off before it started, or cancelled when a cancel came after the credits were taken, which keeps what they cost), with the project, asset, task id, settings, the credits the provider reported (or the price list, marked as estimated), and whether it was a redo or came from an agent. A line that cannot be read is skipped and counted, never fatal. Credits (in the 3D queue and Settings > 3D providers) shows each provider's balance (Tripo `GET /v3/account/balance`, Meshy `GET /openapi/v1/balance`, both free), what went out in the last 7 and 30 days, the share that went on redos, and every spend, filtered by project and provider, with Export CSV.

Provider jobs run on their own lane, four at a time, with 429 back off that honours `Retry-After`. A request that cannot connect at all (a Wi-Fi switch, a dropped link) is tried again for about half a minute, since nothing was sent; a failure says why (for example `tcp connect error: No route to host`), and one that leaves open whether a create arrived says to check the provider's dashboard before sending again. Each job and its remote task ids are saved in `projects/<p>/providers.json`, so a job sent before the app closed resumes polling instead of paying again. Job ids (`<asset>-<n>`) are unique across the workspace, so the same asset name in two projects never shares one; older saved jobs that do are given a fresh id when the workspace opens. The Board card shows a job's progress, and the 3D queue lists every send with its provider, credits and time. A finished model goes into the inbox linked to its asset, so Accept defaults to that asset. A 402 says the account is out of credits.

API keys live in the OS keychain (macOS Keychain, Windows Credential Manager) under the service `com.shironex.maquette`, never in `maquette.json`, a project file or a log. Settings > 3D providers shows only the last four characters, and Test key calls the free balance endpoint. Presets live there too, per provider with a character variant.

</details>

<details>
<summary><strong>Inspector</strong></summary>

The Inspector opens a project's models in the same studio as the preview, with a compare slider that splits one canvas down the middle: the raw export against the shipped build or the site's copy, or two versions of one asset. Both halves share the camera. Camera stops come from the first of these that has any: `stops.json` in the project (`{ "stops": [{ "name", "position": [x, y, z], "target": [x, y, z], "fov": 35 }] }`, glTF coordinates in metres), the stops of the scene layout, the `web/posters/stops.json` the Capture posters step writes, or cameras inside the models. The filmstrip eases the camera to a stop and shows its poster when there is one.

The side panel lists the selected model's triangles, textures with size and codec, a GPU memory estimate, the transfer size and its compression preset with any lock note. Under Compression each map group (color, normal, data) has its own ETC1S or UASTC switch with what that group's maps weigh as built; after a run that changed a codec, the line shows the new size, and until then it says the maps are still in the old one.

The Material section tweaks the selected model per asset: a tint (multiplied into the base color in linear space, the color map stays), roughness set to a value or multiplied, metalness set, and emissive strength (`KHR_materials_emissive_strength`). Applies to picks every material of the model or one by name; a named material's fields win over every material's. The view previews edits on the models at once (in the Shaded look) and marks them Not applied yet; Reset drops them, Apply tweaks saves them (only the edited assets' looks, laid over the file on disk, and only once the saved tweaks have loaded), and quitting with tweaks not applied asks first, like the presets. Setting roughness or metalness replaces the model's metal and roughness map, which holds both. The tweaks live in `materials.json` in the project, keyed by asset id, so they carry over to new model versions. A save that finds a `materials.json` it cannot read, or one from a newer Maquette, is refused and the file is left as it is:

```json
{
  "version": 1,
  "models": {
    "calipers": {
      "all": {
        "tint": "#5d6266",
        "roughness": { "mode": "override", "value": 0.42 },
        "metalness": 0.85,
        "emissiveStrength": null
      },
      "materials": {}
    }
  }
}
```

Compress writes the starting factors of each tweaked material under `extras.maquette.original` in the shipped file, so the preview of a tweaked build starts from those instead of applying the tweaks twice. Budget bars come from the workspace budgets and an optional `budgets.json` in the project (`{ "models": { "character": 1.0 } }`, in MB); models without a row share what is left of the scene budget. The frame time graph measures the renderer (waiting on the GPU every few frames) against the 60 fps line. Export GLB re-exports the selected model alone: it runs the pipeline's Blender export step with its output pointed at a scratch folder, then copies that one file over the raw export.

#### Scene layout

The Layout tab places each asset's newest model and the camera stops in one 3D canvas. Click a model (or pick it in the list) and drag the gizmo: Move, Turn and Scale (W, E, R; scale stays uniform), snapping to 5 cm and 15 degrees unless the magnet is off. The side panel takes exact numbers (metres, degrees, scale), shows the size in centimetres, and has Drop to desk and Reset turn and scale. Undo and redo cover every change (Cmd+Z, Shift+Cmd+Z), F frames every model and Cmd+S saves.

Camera stops are added where the editor camera stands, named, reordered, deleted, moved with the gizmo or Set from view, and each has its field of view. Look through flies the editor camera to a stop and dims what falls outside the 16:9 poster frame. An optional desk sets the slab the models stand on.

Save layout writes `layout.json` in the project, with every model pinned where it stands:

```json
{
  "version": 1,
  "models": { "elsheria": { "position": [1.6, 0, 3.2], "rotation": [0, -25, 0], "scale": 1 } },
  "stops": [
    {
      "id": "arrival",
      "name": "Arrival",
      "position": [6.5, 2.4, 10.5],
      "target": [1, 1.4, 2.6],
      "fov": 36,
      "models": null
    }
  ],
  "desk": { "size": [1.6, 0.9] }
}
```

Positions are glTF metres (Y up), rotations degrees applied X, then Y, then Z, keyed by asset id. Up to 24 stops. The next pipeline run builds the scene, bakes it and renders one poster per stop from it. When an agent changes the layout while you have unsaved edits, a banner offers their version instead of overwriting yours.

The first view is what the first stop loads before the visitor moves: the models in its frame, or the ones ticked under Count what the camera sees (the stop's `models`). Its size is measured against its own budget, First view target under Settings > Budgets (3 MB by default), and shows in the Layout bar, the Pipeline view and the scene budget tooltip in the top bar.

#### Paths

The Paths tab draws curves through the scene that the site shows as glowing tubes: a cable from the trench to a panel, a pipe, a trail. Each path is a list of points in the layout's space (metres, Y up), joined smoothly (Catmull-Rom) or by straight runs. Select picks a point or a path in the 3D view and the gizmo moves the picked point; Add points (A) puts a point where a click lands on a model or the ground, after the picked one; Delete removes the picked point (a path keeps two), F frames the scene and the magnet snaps to 5 cm. The points table takes exact numbers, shows how far along each point is and adds a point at the end. The side panel lists the paths and sets the selected one's name, curve, colour, glow colour and width. Draw on surfaces, end caps, a glow strength, speed and direction show as not built yet.

Either end can follow a model: anchored, it moves by as much as the model's placement moves whenever the layout saves (from the Layout tab or an agent), so a cable stays plugged in. Only the move follows, not a turn or a scale. Energize gives the path a track the Timeline edits: how much of it is lit at camera stops, 0 dark to 1 lit from the first point to the last. Every edit saves into `layout.json` half a second later:

```json
{
  "paths": [
    {
      "id": "rune-line",
      "name": "Rune line",
      "curve": "catmull-rom",
      "points": [
        [-3.2, 0, 3.4],
        [0, 0.05, 5.2],
        [3.6, 0, 1.2]
      ],
      "start": "observatory-arch",
      "end": "crystal-lantern",
      "width": 0.06,
      "color": "#b89cff",
      "glow": "#e2d6ff"
    }
  ],
  "timeline": {
    "paths": {
      "rune-line": {
        "easing": "linear",
        "keys": [
          { "stop": "veil-gate", "amount": 0 },
          { "stop": "keystone", "amount": 1 }
        ]
      }
    }
  }
}
```

Up to 32 paths of 2 to 128 points, widths from 1 mm to 1 m. The site builds each path as one tube mesh (8 sides, a ring about every two widths, 16 to 256 rings) and one draw call, and a shader lights it from its first point with a soft front and a bright head. Each path is a few hundred bytes of `layout.json`, so they are not counted in the scene budget, and the Build scene step and its posters do not draw them yet.

</details>

<details>
<summary><strong>Motion</strong></summary>

Motion (the walking figure in the rail) lists the project's models that move: a rig, animation clips or object states. The ones that do not move yet sit under No motion yet, and a project where nothing moves shows what makes a model show here and suggests the prop with the most separate parts for states.

**Clips.** Every clip comes from the model file as it is (Maquette reads the GLB: name, frames, key rate, keys, channels, keyframe bytes, how far the root travels and whether the last key matches the first; a model packed by meshopt says those two are not readable). Pick a model and a clip to play it in the studio with the skeleton, the dashed path its root walks and a turntable. The timeline plays, steps a frame (`,` and `.`), sets a preview loop (`I` and `O`, or drag the white handles), loops (`L`) and changes the speed; the loop only previews, the clip ships whole. Each clip has a ship switch: one switched off stays in the model but is left out of the site from the next Build scene. Names, rates and compression are the file's own, so they are read, not edited. A rig without clips offers the three ways to a first one: library clips (needs a rig; Use on model maps and bakes them, see Libraries), a `.glb` or `.blend` dropped in the Inbox as the model's next version, or a built asset script that bakes one headless.

**Rig check.** The bone count and whether it maps onto the humanoid (Mixamo) skeleton, the most bones one vertex weighs on against the web's four, and the humanoid bones the rig is missing, with Map bones opening Use on model at its bone map: each humanoid bone, the rig bone it matched (exact, by a name other rigs use, or saved for the model) or none, each one changeable.

**Budget.** Motion has no budget of its own: clips are bytes inside the models, so the top bar shows the scene budget with the draft's ship choices in it, and the tooltip says how much of it is clips and how much of that loads with the first camera stop. Over budget, the list and the clip rows mark the clips that would have to stay off for the scene to fit, heaviest first, with Leave it off for each and Open Pipeline for packing keys with meshopt.

**States.** A prop's states are poses of its parts (the mesh nodes of its file): the first is the model at rest, every other moves named parts by an offset in metres and degrees from rest. Between two states a transition has a duration, delay, easing and stagger; triggers say what starts a move on the site (arriving at a camera stop, a scroll range, a click or a hover, on the whole model or one part). The States tab poses the parts, plays and scrubs the move into the selected state with the moved parts outlined, and edits the timing and the triggers. Save motion writes the states and the clip choices into `layout.json`, next to the camera stops:

```json
{
  "states": {
    "switch-cabinet": {
      "states": [
        { "id": "closed", "name": "Closed, off", "parts": {} },
        {
          "id": "door-open",
          "name": "Door open",
          "parts": { "Door": { "position": [0, 0, 0], "rotation": [0, -105, 0] } }
        }
      ],
      "transitions": [
        {
          "from": "closed",
          "to": "door-open",
          "duration": 0.6,
          "delay": 0,
          "easing": "ease-out",
          "stagger": 0
        }
      ],
      "triggers": [
        {
          "kind": "stop",
          "to": "door-open",
          "stop": "porch",
          "part": null,
          "start": null,
          "end": null
        }
      ]
    }
  },
  "clips": { "clay-figure": { "Wave": { "ship": false } } }
}
```

Up to 16 states and 32 triggers per asset. Layouts written before Motion load as they are. The site plays the stop and scroll triggers as the page scrolls (see Timeline); click and hover travel with `layout.json` (and with the site data Ship writes) for a page to read, and the site does not play them yet.

</details>

<details>
<summary><strong>Timeline</strong></summary>

The Timeline (the film strip in the rail) lays out what happens as the page scrolls. Its columns are the project's camera stops; the site's stop sections are one window high each, so stop `i` of `n` sits at `i / (n - 1)` of the page, and the ruler counts the page in percent, pixels (once the page answers the bridge with its scroll) or sections. Above the lanes is the Page view's frame, started the same way: drag the ruler, press `[` and `]` to jump a stop or `Space` to play at reading speed, and the page scrolls along through the page bridge. A project without camera stops shows where to add them.

- **Camera.** A stop at each one's share of the page and the flight between them; click one to jump there.
- **States.** Every prop with states in Motion gets a lane of its stop and scroll triggers: a pill where arriving at a stop plays the move once (its transition's duration, delay, easing and stagger), a block where a scroll range scrubs it. A plus at a stop adds a change there to the next state. The side panel switches a change between a camera stop and a scroll range, picks its state, stop or range, and removes it; a timed change shows as not built yet. Click and hover triggers are left as Motion has them.
- **Explode.** Add track > Explode gives a model a track of amounts at camera stops, from 0 (together) to 1 (each part `distance` metres out from the model's centre), eased between the keyed stops and held outside them.
- **Paths.** Every path with an energize track gets a lane of amounts at camera stops, from 0 (dark) to 1 (lit from end to end), eased between the keyed stops. Add track > Energize gives a path a track, and New path... opens the Inspector's Paths tab to draw one.

The checks warn about a change that changes nothing, two ranges of one prop that overlap, an explode track that never moves and a path that never fills. Every edit saves into `layout.json` half a second later. The state changes stay the props' triggers, and the explode and energize tracks go under `timeline` (`explode` and `paths`, see Paths above):

```json
{
  "timeline": {
    "explode": {
      "switch-cabinet": {
        "distance": 0.3,
        "easing": "ease-in-out",
        "keys": [
          { "stop": "canopy", "amount": 0 },
          { "stop": "pullback", "amount": 1 }
        ]
      }
    }
  }
}
```

Ship copies the layout into the site whole, and `apps/site` plays it: `scene/timeline.ts` turns tour progress into each prop's pose and each track's amount, and `scene/tracks.ts` moves the model's nodes by the names in the file; `scene/PathTubes.tsx` draws the paths and lights each by its track. With reduced motion the site shows its posters instead of the 3D scene. Agents set the tracks with `set_layout` (`explode` and `energize`) and the changes with its `states`.

</details>

<details>
<summary><strong>Page view</strong></summary>

The Page view (Page in the rail) shows the project's website with its models live in it, the way a visitor sees it. It runs the site's dev server itself: it picks a free port, runs the site's dev command in the site repo, shows the server starting with its log, and stops it when you leave the view or quit. The command comes from Settings > Project (Site dev command, where `{port}` stands for the port Maquette picks), else from `maquette.site.json`, else from the site's `package.json` scripts and lockfile; it runs as a program with its args, never through a shell. A server Preview on site or an agent started is reused and left running; one already answering at the site's address is never stopped. Without a site repo, with no dev script, or when the server fails, the frame says why and offers Settings, Try again or a page concept.

Most sites forbid being framed: Next.js headers, Helmet and many hosts send `X-Frame-Options: DENY` or `Content-Security-Policy: frame-ancestors 'none'`, and the browser then leaves the frame blank. So the frame does not load the dev server directly but a small frame proxy Maquette runs on `127.0.0.1` in front of it, on a free port, for as long as the server runs. The proxy forwards every request to that one dev server (it refuses anything not on this machine), drops `X-Frame-Options` and only the `frame-ancestors` directive of the CSP (and of `Content-Security-Policy-Report-Only`), and leaves every other header, directive and body as the site sent it. It streams responses, passes WebSockets through so hot reload keeps working, sends the dev server's own `Host`, and points redirects on the dev server back at itself. Nothing in the site changes. The address bar, the status bar and Open in browser show the dev server's real address, not the proxy's.

- **Preview.** The page in a browser frame at Phone (390 x 844), Laptop (1280 x 800) or Desktop (1440 x 900) width, scaled to fit. The scroll scrubber below lists the page's camera stops; drag it, step it or play it and the page scrolls along. Stops pins the current stop on the frame and Poster shows the page as it is without WebGL. The side shows the frame time the page measures, the scene on the page (the site repo's models, the current ship) against the scene budget, GPU memory and draw calls against the device budget, the current ship against this run, and the source.
- **Ship vs run, With poster.** Compare (in the side) puts the current ship, live, beside this run's poster for the current stop with its numbers and what changed, and With poster adds the page without WebGL. The side then gives the device's verdict: this run against the device's frame time, first view, GPU memory and draw call budgets, and what changed per model since the ship. This run is shown as its poster still: the site only shows a run once Ship copies it there.
- **Devices.** The same page at all three widths at once, each following the scrubber with its own frame time.
- **Page concept.** Instead of the site, an `.html` file inside the workspace (one the Lab made, for example), shown as it is with no dev server.
- **Open in browser** opens the page on the dev server itself.

The scrubber talks to the page through a small window message bridge, which works the same through the frame proxy: the page's own origin is then the proxy's, and it answers whatever origin said hello. A site does not need to add it: the frame proxy adds Maquette's bridge shim to every page it frames (a script it serves itself at `/__maquette/bridge.js`, so a CSP with `script-src 'self'` allows it). The shim scrolls to the element whose `id` is a stop's slug or whose `data-maquette-stop` names it, else spreads the project's camera stops evenly over the page's scroll, follows the page's own scrolling and measures frame times; draw calls stay estimated. The scrubber and the readouts say whether the site's bridge or the shim answered. A page with its own bridge sets `window.__maquetteBridge = 1` and the shim stays out of its way; Maquette also prefers any bridge that answers without `source: 'shim'`. To speak it, a page listens for messages with `maquette: 1`:

```js
// Maquette to the page
{ maquette: 1, type: 'hello', stops: [{ name: 'Porch' }] } // after the frame loads, until the page answers; the project's camera stops
{ maquette: 1, type: 'goto', stop: 2.5, smooth: true }  // fractional stop: 2.5 is halfway from stop 3 to 4
// The page to Maquette (to the origin that said hello)
{ maquette: 1, type: 'ready', stops: [{ name: 'Hero' }], features: ['goto', 'at', 'stats', 'poster'] }
{ maquette: 1, type: 'at', stop: 2.5, scroll: { top: 900, max: 3600 } }
{ maquette: 1, type: 'stats', frameMs: 9.4, p95Ms: 12.1, drawCalls: 86 } // about once a second
```

With `?maquette-poster=1` in its address the page shows its no WebGL version. `apps/site/src/scene/bridge.ts` is a complete page side.

</details>

<details>
<summary><strong>Libraries</strong></summary>

The rail's Libraries group holds Models and Animations, each with its count. Models (for website and game projects) keeps finished models outside every project, in `library/` in the workspace root (`~/Maquette/library` by default). Each entry is a folder with `entry.json` (name, kind, tags, refs, versions), `models/` (one file per version), `thumbs/` and `refs/`. Add to library (a Board card's menu, the Inspector, or Add from a project in the view) copies an asset's newest model, picture, refs, real size, fix-ups and material tweaks; the same hash adds nothing and a newer model adds the next version. Use in project copies a version into a project as a new asset at v1 and spends no credits; a website project refuses an `.obj`, and a game project takes either and writes the game's format on the next Game export. The library copies and offers updates, it never links: an asset copied from an older version shows Update to vN, and the dialog lists what carries over and what no longer applies before anything changes. The view searches, filters by tag, source, format and project, and shows the credits reuse saved.

Animations keeps clips once in `library/animations/`, one folder per clip with `clip.json` (name, pack, tags, source rig with its bones and rest pose, frames, rate, length, root motion, loop, size, origin) and `clip.glb` (skeleton and keys, no mesh). Import clips reads Mixamo FBX files and pack zips headless in Blender: each FBX is one clip, the skin is dropped, the rig is read from the bone names (Mixamo, Tripo or custom), duplicates are skipped or kept both, and files with no keys are left out. Save clips to library (Motion's Add clip menu, or the Inbox for a rigged model with clips, such as a Tripo job's animations) saves a model's clips as a pack, never the ones that came from the library. Use on model (or Motion's Add from library) bakes clips onto a rigged model in four steps: choose clips, a bone map saved per model (Mixamo and Tripo names suggested, never blocking), per model clip settings (snake_case name, trim, loop, in place or root motion, speed), and a review of a headless Blender bake on a copy, each clip good, to look at (a planted foot slides) or failed (left out); baking adds the model's next version and keeps the old one. The project's `animations.json` records what each model baked, so a clip fixed by a later import offers those models an update. Mixamo clips may be used in your projects but not handed out as raw files; the library stays on this machine and is never shipped.

</details>

<details>
<summary><strong>Sync with another computer</strong></summary>

A workspace can live in a Syncthing folder shared by two computers, for example a Mac and a Windows PC. What belongs to one computer stays in its app data: settings, API keys, the Downloads watch's seen list and where each project's site or game repo is. `project.json` keeps only the repo's folder name, so the other computer asks once "Where is this project's repo on this computer?". Handoff folders and page concepts are saved relative to the workspace.

While Maquette has a workspace open it keeps `.maquette-lock.json` in it (host, OS, app version, a heartbeat every minute) and removes it on a clean exit. Opening a workspace whose lock another computer refreshed in the last three minutes asks first; an older lock is taken over without a word. Every scan skips Syncthing's own files, and when Syncthing keeps a `*.sync-conflict-*` copy of a Maquette data file the app names it and merges nothing.

Settings > Workspace > Sync with another computer has the recommended `.stignore` (the cache and OS clutter, never the lock file, which is how the other computer knows) and a copy button. Turn on staggered file versioning for the folder, and use Maquette on one computer at a time.

</details>

## Stack

- **Shell:** Tauri 2 (`apps/desktop`), macOS and Windows
- **UI:** React 19, Vite, Tailwind CSS v4, zustand (`apps/web`)
- **Kit:** typed components with a theme generated from `tokens.json`, Phosphor icons, Geist fonts bundled (`packages/ui`)
- **Core:** Rust domain logic (`crates/maquette-core`): tool detection and `--version` parsing, the `maquette.json` workspace config, projects, assets, the pipeline runner and cache, GLB analysis, the inbox watcher, concept jobs, provider jobs for Tripo and Meshy (reqwest with rustls) and the inspector's numbers
- **MCP:** rmcp (the official Rust SDK) served by hyper in the desktop shell
- **Viewer:** three.js with react-three-fiber for the model preview
- **IPC:** tauri-specta writes typed bindings to `apps/web/src/ipc/generated/bindings.ts` when `cargo test -p maquette-desktop` runs

[docs/architecture.md](docs/architecture.md) describes the layout: features with one folder per component, which features may import which, the lint tiers and the Rust lint policy.

## License

The code is licensed under the [GNU Affero General Public License v3.0 only](LICENSE). The artwork in this repository (the Frostveil sample, the template and welcome pictures, the landing page scene, the README screenshots and the Maquette name and icon) is not: all rights are reserved, see [ARTWORK.md](ARTWORK.md).
