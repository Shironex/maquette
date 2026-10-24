# Maquette

A desktop studio that turns AI concept art into real-time 3D web scenes. One board follows each asset through the pipeline: Concept, Refs, Tripo, Blender, Web, Ship. Everything runs on this machine.

So far: the app shell, onboarding, the toolchain check, workspace config and Settings, an asset Board, the pipeline runner, the model inbox, the Concept lab, Send to 3D (Tripo, Meshy or by hand), the Inspector with its scene layout editor and a built in MCP server for agents.

## Projects

A workspace holds any number of projects. The project switcher at the top of the rail (or Cmd+P on macOS, Ctrl+P on Windows and Linux) opens a menu of them with their picture, template, asset count and age; the open one is checked, and a search field shows once there are more than six. Arrow keys move, Enter opens, Escape closes. New project opens the onboarding project form in a dialog (template, name, target site repo) and Import folder the import form. The open project is saved as `currentProject` in `maquette.json`.

Switching starts every view of the new project clean: Board, Lab, Pipeline, Inspector, the 3D queue and the status bar. Runs, Lab images and provider jobs keep going in the project they belong to, and the menu says what is still running there. The Inbox is shared by the whole workspace: a model sent from another project names that project under Sent for, and Accept moves it into the project it was sent for. The rail collapses to icons (Collapse at its foot); the switcher and its menu work there too.

Copy to project... in a Board card's menu copies the asset into another project and leaves it where it is: its refs, every model version, thumbnails and finished sends go along, renamed to the new id. When the other project already has an asset by that name the copy is called "Name 2". A send still running stays with the original.

In the Lab, an earlier direction's menu has Move to project..., which moves its folder (concepts, picks, stars and brief) into another project. The refs it sent and its Board card go along when the card has no model yet; a card with models, or one with a send under way, stays where it is.

## Pipeline runner

Each project keeps its steps in `pipeline.json` (new projects start from the Floating diorama template: Build, Bake light, Export GLB, Compress, Capture posters, Ship). A step has an `id`, a `kind` (`blender`, `shell`, `gltf-transform`, `toktx`, `ffmpeg`), a script or command, an args array, `inputs` and `outputs` globs and `depends_on`.

- Programs are spawned directly with an args array, never through a shell, on the same PATH the toolchain check used. On Windows they start with `CREATE_NO_WINDOW` inside a job object, so Cancel ends the whole process tree.
- Blender always runs in the background: a `blender` step without `-b` is refused. The only time Maquette opens Blender's window is when you click Open in Blender.
- A step is skipped when its input files, script, args and tool version hash to the same key as the last good run (kept under `cache/`). Run from here starts at a step; Ignore the cache reruns everything.
- One run at a time per project; more runs queue. Runs and their logs (last 5) live in the project's cache folder.
- The Compress step reads the per model presets (texture size, ETC1S or UASTC, meshopt level). A locked row keeps its values and says why. A scene project gets the locked character row only once it has a character asset; a Character showcase starts with it. Rows read as the asset a model was exported from (`clay_v1.glb` is "clay").
- A running node shows a progress bar when its tool reports how far it is: a script's `progress N/M` or `progress P%` line (with an optional `[tag] ` prefix), Cycles' `Sample N/M` inside the current item, or one finished call of several (Compress runs one chain per model). Otherwise the bar is indeterminate.

### Starter scripts

A new project gets small, commented scripts in its `tools/` folder when its pipeline is first written, so it runs end to end on day one. They read their paths from args and are the project's to edit; Maquette never writes over a file that exists on its own.

`tools/.starter.json` records the hash of every starter Maquette wrote, so a later version can tell an untouched copy of an older starter from one you edited (scripts from before that record are matched against the known older starters). When newer starters ship, the Pipeline view says New starter scripts available and Update brings the untouched ones up to date in one go. An edited script is never overwritten silently: it shows as edited, with Replace with the new starter (your file is kept as `<name>.py.bak`) or Keep mine.

- `build_scene.py`: opens `blender/scene.blend` when there is one (else an empty scene with a sun), imports each `models/*.glb` into its own collection under "Models", places them from the scene layout (`layout.json`, below) and saves `blender/scene_built.blend`. Models without a placement that overlap are laid out side by side. The layout's desk becomes a slab under the models, so they shadow it in the bake, and its camera stops become cameras in a "Stops" collection unless the base blend has stop cameras of its own.
- `bake_light.py`: gives every mesh outside "Studio" a second UV named `lightmap` (Smart UV Project when it is missing) and bakes Cycles diffuse light (direct and indirect, or the full look with `--type combined`) into one `blender/bake/<object>_light.png` per object, on the CPU. The defaults are quick (`--size 512 --samples 16`); raise them on the step's args for a final bake. The whole scene bakes together, so every model shadows the others and the desk. Each map stores light divided by a scale of at least 1; `bake.json` lists every object's file, size and scale. It prints `progress N/M` per object and saves `blender/scene_baked.blend`. A scene without meshes writes `"baked": false` and passes.
- `export_web.py`: one GLB per model collection (and per other top level object) into `web/raw/`, named in snake case, uncompressed for the Compress step. A baked object keeps both UV sets (`lightmap` is `TEXCOORD_1`), carries its lightmap as the emissive texture on that UV and has the material extras `{"lightmap": <scale>, "lightmapUv": 1}`, so a page can show albedo times lightmap times scale in an unlit material.
- `render_stops.py`: one poster per camera stop, a camera with the `maquette_stop` custom property (its number sets the order) or a name starting with `stop_`. A scene without stops gets one framed three quarter poster of the whole scene instead (`overview.png`, from a temporary camera fitted to the scene bounds that is never saved into the blend), and the log says so. It always writes `web/posters/stops.json`, with each stop's name from the layout as its `label`, and removes the posters the previous one listed.
- `ship.mjs`: copies `web/dist/*.glb` to `public/models` and the posters to `public/posters` in the project's site repo. Without a site repo it says so and passes.

Projects made before the layout editor get these `build_scene.py` and `render_stops.py` through the same Update banner when their copies are untouched; the Build scene step then reads `layout.json` too, so saving a layout reruns it.

Character showcase projects get `build_character.py`, `export_character.py`, `render_stops.py` and `ship.mjs`. The build imports the newest `models/*.glb` (highest `_v<n>`, then newest file) with its rig and clips, on top of `blender/character.blend` when you add one, adds a key and fill light when the scene has none and a front camera stop `stop_01_front` when it has no stop, and saves `blender/character_built.blend`. Export GLB and Capture posters read that file, so the pipeline runs as soon as a character is in `models/`.

## Model inbox

Maquette watches the workspace `inbox/` folder for `.glb` and `.gltf` files. Each file is read in Rust: triangles, meshes, materials, textures and their sizes, skins and joints, animations. Checks compare that with the budgets: triangles, texture size, a Mixamo rig for characters, PBR maps, separate objects merged into one mesh, and hair or tail vertices weighted to finger bones. Accept to Blender moves the file into the project's `models/` folder as a new asset version.

Two checks are heuristics:

- **Merged objects:** a mesh whose welded triangles form several islands far apart (gaps over 10% of the mesh size, each island at least 1% of it) probably holds objects that were separate. Scenes are exempt.
- **Weighted to finger bones:** a vertex whose strongest joint is a finger bone, but which sits more than 15% of the model height away from that bone (or above the head joint), is counted. Draco or meshopt compressed geometry skips both checks.

The preview stands every model in the same studio: a graphite backdrop, a soft floor with a contact shadow, a warm key, a warm fill and a cool rim light, and a three quarter camera framed to the bounds. Shaded, wireframe over clay, a numbered UV checker, and the skeleton drawn in clay over a ghost of the mesh. It reads meshopt geometry and KTX2 textures (the Basis transcoder ships in `apps/web/public/basis`).

### Thumbnails

Inbox models get a 256 px thumbnail when they arrive, and Board assets get one on Accept and for each new version. A headless Blender job (always `blender -b`, never the GUI) renders the model in the studio above, on the runner's own `thumbnails` lane, so renders queue and never block the UI. Renders are cached by content hash in `cache/thumbs/` and copied to `<project>/thumbs/<asset>_v<n>.png`. When Blender is missing, or cannot read the file (KTX2 textures), the UI draws the model once offscreen with three.js in the same studio and stores that instead. While a render runs, cards and rows show the asset kind's icon.

A compressed model is rendered from an uncompressed copy, because Blender reads neither KTX2 textures nor meshopt geometry. First match wins: the file itself when Blender reads it; else its uncompressed export (`../raw/<name>` next to a `dist` folder, then `raw/<name>`, then the project's `web/raw/<stem>.glb`); else the three.js snapshot above.

Render thumbnails by hand with `cargo run -p maquette-core --example render_thumbnail -- <thumbs dir> <model>...`.

## Concept lab

The Lab turns a brief into concept images with the Codex CLI. A brief is a direction name, a prompt, up to 8 reference images (dropped in, picked from the project's `refs/`, or chosen from a file dialog), an aspect ratio and 1 to 12 images. Each image is one Codex job:

```sh
codex exec --skip-git-repo-check -s workspace-write -i <ref> -i <ref> -- -
```

The prompt goes on standard input, starts with "Generate an image." and ends by asking Codex to copy the result out of `~/.codex/generated_images/` to an exact path, `concepts/<direction>/<direction>_NN.png` in the project. `-i` takes several values, so `--` ends the list before the `-` that reads the prompt. Jobs run in the project folder on their own lane, 4 at a time by default (Settings, `lab.concurrency`, up to 8), and each is stopped after `lab.timeoutMinutes` (10). A job succeeds when the file appears. If Codex saved the image but not the copy, the newest image from that Codex session is picked up instead; otherwise the job fails with the last error line of its log.

Results show per direction with Star and Pick and a lightbox (arrow keys, S, P, Escape). Contact sheet tiles every image with ffmpeg into `contact_sheet.png` next to them. Send picks to Refs copies the picked images into `refs/` and moves the asset with the direction's name to Refs on the Board, or adds it there. The brief and marks live in `concepts/<direction>/direction.json`.

Rename... in a Board card's menu changes only the name on the card: the asset's id, files and versions keep their names, so refs, inbox matches and pipeline outputs stay linked.

Make object refs (on a Lab image, or in the menu of a Refs card whose picture is a ref) turns one concept into a clean reference image per object, for a scene that needs many props. List the objects as chips (Enter, a comma, or a pasted list with one per line; up to 12, each name 60 characters at most, shown with a counter and never cut), click a chip to rename it or add notes for Codex (they steer the image and are not part of the asset name; `make_object_refs` takes them as `notes`), pick the aspect (1:1 by default) and whether they become props or characters. Each object is one Codex job on the same lane, with the concept attached as `-i`, and a built in prompt asks for that object alone: centered, the whole of it in frame, a front three-quarter view, a plain light grey background, even soft light, no cast shadow on the background, no text, and the concept's materials and style. The images land in a new direction, "<name> objects", each titled with its object. Send picks to Refs there copies every pick to `refs/<object>.png` and makes one asset per pick, named after the object; Send to 3D does that and opens the batch panel for all of them.

`cargo run -p maquette-core --example generate_concept -- <project dir> --direction "Stone lantern" --prompt "..." [--ref refs/a.png] [--count 1]` runs the same jobs from a terminal. Every image spends Codex usage.

## Send to 3D

Send to 3D takes refs from a Board card (its menu) or the picked concepts of a Lab direction (the picks go to Refs first) to Tripo, Meshy or By hand. The panel starts from the Settings preset for the asset (the prop or the character variant, picked by asset kind), and every field can change for this send only. One image makes one model; with Multi-view on, up to four images go as front, left, back and right.

- **Tripo** (REST v3): upload, image to model or multiview to model, poll, download. A character can add the rig chain: rig check, rig, then retarget the chosen animations. Tripo has no cancel, so a sent task always finishes and lands in the Inbox.
- **Meshy** (REST v1): image to 3D or multi image to 3D (images go up as data URIs), poll, then rigging for characters. Cancel works while the task is still pending; once Meshy has started it answers 409 and the queue says so.
- **By hand:** no API call and no credits. Maquette writes `projects/<p>/handoff/<asset>-<n>/` with the images named by view and a settings card (`settings.md` and `settings.json`) for the service's web app. The panel shows the same card with copy buttons, Open Tripo studio (or Open Meshy) in the system browser, Open folder, and Reveal image for each image, so it can be dragged into the browser. The asset waits on the Board until the model comes back: Add model... on the card or in the panel (a `.glb`, or the `.zip` a web app downloads, which must hold one `.glb`), drop either on the card, or save it into the inbox as `<asset>-<n>.glb`. Check Downloads looks in the Downloads folder (the OS known folder) for `.glb` and `.zip` files newer than the oldest waiting handoff and suggests a waiting send for each, by file name and time; nothing is taken until you press Use on a row. Every way copies the model into the inbox, leaves the original where it is, and runs the Inbox checks.

Several assets go in one batch: tick the boxes on the Board cards (Shift selects a range) and press Send to 3D in the selection bar, or Send to 3D in an objects direction. One provider and one copy of the presets cover the batch, each asset keeps its own kind (prop or character), and each gets its own job and model. A Tripo or Meshy batch is checked whole first and asks once, listing the task count and settings, before anything is spent. A By hand batch writes one handoff folder per asset and opens the By hand checklist: every handoff of the project grouped by batch, with its settings, folder, status, Add model... and Check Downloads. The By hand button on the Board opens it again.

Provider jobs run on their own lane, four at a time, with 429 back off that honours `Retry-After`. Each job and its remote task ids are saved in `projects/<p>/providers.json`, so a job sent before the app closed resumes polling instead of paying again. The Board card shows a job's progress, and the 3D queue lists every send with its provider, credits and time. A finished model goes into the inbox linked to its asset, so Accept defaults to that asset. A 402 says the account is out of credits.

API keys live in the OS keychain (macOS Keychain, Windows Credential Manager) under the service `com.shironex.maquette`, never in `maquette.json`, a project file or a log. Settings > 3D providers shows only the last four characters, and Test key calls the free balance endpoint. Presets live there too, per provider with a character variant.

## Inspector

The Inspector opens a project's models in the same studio as the preview, with a compare slider that splits one canvas down the middle: the raw export against the shipped build or the site's copy, or two versions of one asset. Both halves share the camera. Camera stops come from the first of these that has any: `stops.json` in the project (`{ "stops": [{ "name", "position": [x, y, z], "target": [x, y, z], "fov": 35 }] }`, glTF coordinates in metres), the stops of the scene layout, the `web/posters/stops.json` the Capture posters step writes, or cameras inside the models. The filmstrip eases the camera to a stop and shows its poster when there is one.

The side panel lists the selected model's triangles, textures with size and codec, a GPU memory estimate, the transfer size and its compression preset with any lock note. Budget bars come from the workspace budgets and an optional `budgets.json` in the project (`{ "models": { "character": 1.0 } }`, in MB); models without a row share what is left of the scene budget. The frame time graph measures the renderer (waiting on the GPU every few frames) against the 60 fps line. Export GLB re-exports the selected model alone: it runs the pipeline's Blender export step with its output pointed at a scratch folder, then copies that one file over the raw export.

### Scene layout

The Layout tab places each asset's newest model and the camera stops in one 3D canvas. Click a model (or pick it in the list) and drag the gizmo: Move, Turn and Scale (W, E, R; scale stays uniform), snapping to 5 cm and 15 degrees unless the magnet is off. The side panel takes exact numbers (metres, degrees, scale), shows the size in centimetres, and has Drop to desk and Reset turn and scale. Undo and redo cover every change (Cmd+Z, Shift+Cmd+Z), F frames every model and Cmd+S saves.

Camera stops are added where the editor camera stands, named, reordered, deleted, moved with the gizmo or Set from view, and each has its field of view. Look through flies the editor camera to a stop and dims what falls outside the 16:9 poster frame. An optional desk sets the slab the models stand on.

Save layout writes `layout.json` in the project, with every model pinned where it stands:

```json
{
  "version": 1,
  "models": { "character": { "position": [1.6, 0, 3.2], "rotation": [0, -25, 0], "scale": 1 } },
  "stops": [
    {
      "id": "porch",
      "name": "Porch",
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

Tools: `list_projects` (each project's id, name, template, site repo, created date, asset count, picture, what runs in it and which one is open in the app), `get_project`, `list_assets`, `get_pipeline`, `set_compression_presets` (per model texture size, data map size, codec and meshopt; only the given fields change, locked rows refuse, the Pipeline view updates live and nothing is spent), `get_run` (status and log tail), `list_inbox`, `get_inbox_checks`, `run_pipeline` (optionally from a step, optionally ignoring the cache), `cancel_run`, `accept_inbox_model` (defaults to the asset the model was sent for), `render_thumbnail`, `start_concepts`, `get_concepts` (a direction's images, stars, picks and job states), `send_to_3d`, `send_batch_to_3d` (several assets under one approval), `make_object_refs`, `get_provider_job`, `get_layout` (the layout, every model with its bounds and size, and the first view), `set_layout` (changes only what it names: `models` as `{ asset, position?, rotation?, scale?, remove? }`, `stops` as `{ id?, name?, position?, target?, fov?, models?, remove? }`, with no `id` adding a stop, plus `stopOrder` and `desk`; the whole result is checked before anything is written and the app's Layout view updates live) and `copy_asset` (`{ project, asset, to }`, the same copy as Copy to project...). The agent rules hold by construction: Blender only runs headless, no tool opens its window, and there is no screenshot or screen capture tool. With Ask before shipping on, a run started over MCP stops before its Ship step and the app shows an Approve or Deny prompt; until then `run_pipeline` and `get_run` answer `waiting for approval`. A run whose Ship step would come from the cache does not ask. `send_to_3d` to Tripo or Meshy always waits for Approve in the app before any credit is spent, whatever that setting says, and Deny cancels the job; `send_batch_to_3d` shows one prompt that lists every asset with its settings and the total, and Approve or Deny covers them all; By hand spends nothing, so it writes the handoff folder straight away. Settings overrides are checked by name and an unknown one is refused with the valid list. Tools that work on a project take its id as `project`; none of them changes the project open in the app. Every call shows in the activity list under Settings > Agents.

## Stack

- **Shell:** Tauri 2 (`apps/desktop`), macOS and Windows
- **UI:** React 19, Vite, Tailwind CSS v4, zustand (`apps/web`)
- **Kit:** typed components with a theme generated from `tokens.json`, Phosphor icons, Geist fonts bundled (`packages/ui`)
- **Core:** Rust domain logic (`crates/maquette-core`): tool detection and `--version` parsing, the `maquette.json` workspace config, projects, assets, the pipeline runner and cache, GLB analysis, the inbox watcher, concept jobs, provider jobs for Tripo and Meshy (reqwest with rustls) and the inspector's numbers
- **MCP:** rmcp (the official Rust SDK) served by hyper in the desktop shell
- **Viewer:** three.js with react-three-fiber for the model preview
- **IPC:** tauri-specta writes typed bindings to `apps/web/src/ipc/generated/bindings.ts` when `cargo test -p maquette-desktop` runs

## Development

Needs Node 22.12 or newer, pnpm 10 and the Rust toolchain pinned in `rust-toolchain.toml`.

```sh
pnpm install
pnpm dev        # the desktop app with hot reload
pnpm dev:web    # the UI alone in a browser, in mock mode
```

### Mock mode

Outside Tauri, a dev build answers every command from an in-memory backend (`apps/web/src/ipc/mock`), so the whole UI runs in a plain browser at `http://localhost:15190`. Pick a scenario with a query parameter:

- `?scenario=first-run` (default): no workspace yet, toktx missing
- `?scenario=configured`: a workspace with Moon Island open, its last run failed at Capture posters
- `?scenario=running`: like configured, with run 25 streaming and held at Export GLB
- `?scenario=idle`: like configured, the last run passed
- `?scenario=no-blender`: Blender missing, so setup stops at the toolchain step
- `?delay=700`: how long a toolchain check takes, in milliseconds
- `?pace=260`: milliseconds between streamed log lines in a mock run
- `?lab=running`: two concept images of the porch direction mid job; `?conceptMs=4000` sets how long a fake image takes
- `?agents=active`: the MCP server on, with an agent's activity, a run waiting at Ship and a Send to 3D waiting for approval
- `?agents=batch`: like `active`, with three sends to Tripo waiting under one approval
- `?handoff=batch`: adds the landing workbench concept, the six object refs made from it (an objects direction) and a By hand batch of them in mixed states; Check Downloads finds two matches and a stranger
- `?projects=many`: eight projects instead of three (Moon Island, Rooftop cinema, Lantern keeper), so the switcher shows its search field

Configured scenarios also open with fake provider jobs: Sakura generating at Tripo, Stone lanterns waiting for a model by hand, Far spire failed at Meshy for credits, and Character accepted. Only the Tripo key is saved. New sends move along on timers; nothing is sent anywhere.

The mock runner streams Blender style output and fails Capture posters once. The mock inbox lists a character, two props and a broken download. Its previews load real meshopt and KTX2 models, and the Board and Inbox show real studio thumbnails, all from `apps/web/src/assets/mock` (see the README there: that artwork is reserved and not covered by the repository license). For hand testing a real inbox, `cargo run -p maquette-core --example write_fixtures -- <dir>` writes small generated GLBs.

### End to end check

`cargo run -p maquette-core --example new_project -- <workspace dir> <name> [--site <repo>]` creates a workspace (if needed) and a project with its pipeline and starter scripts, the way the app does. `cargo run -p maquette-core --example run_pipeline -- <project dir>` then runs a real project's pipeline from the terminal with the same runner the app uses, fills `{workspace}` and `{siteRepo}` like the app, and prints each step's state. A second run with nothing changed reports every step cached except Ship, which writes outside the project and always runs.

## Scripts

| Script              | What it does                                                                                                                                                                                                                   |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `pnpm lint`         | ESLint over the workspace                                                                                                                                                                                                      |
| `pnpm typecheck`    | TypeScript in every package                                                                                                                                                                                                    |
| `pnpm test`         | Vitest for `apps/web` and `packages/ui`                                                                                                                                                                                        |
| `pnpm format:check` | Prettier check (`pnpm format` writes)                                                                                                                                                                                          |
| `pnpm rust:fmt`     | `cargo fmt --check`                                                                                                                                                                                                            |
| `pnpm rust:lint`    | `cargo clippy` with warnings as errors                                                                                                                                                                                         |
| `pnpm rust:test`    | `cargo test` for the workspace, which also regenerates the IPC bindings                                                                                                                                                        |
| `pnpm gate`         | All of the above, in order                                                                                                                                                                                                     |
| `pnpm build`        | Release build of the desktop app                                                                                                                                                                                               |
| `pnpm visual`       | Screenshots of mock mode in headless Chromium (set `PLAYWRIGHT` to a local Playwright `index.mjs` and `OUT_DIR` for the output; `ONLY=handoff` renders only the object refs and batch shots, `PORT` picks the dev server port) |

Commits follow Conventional Commits with a required scope (`desktop`, `web`, `ui`, `core`, `workspace`, `deps`, `docs`, `scripts`).

## License

All rights reserved. See [LICENSE](LICENSE).
