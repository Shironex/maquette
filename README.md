# Maquette

A desktop studio that turns AI concept art into real-time 3D web scenes. One board follows each asset through the pipeline: Concept, Refs, Tripo, Blender, Web, Ship. Everything runs on this machine.

So far: the app shell, onboarding, the toolchain check, workspace config and Settings, an asset Board, the pipeline runner and the model inbox. The Concept lab, the 3D inspector and the MCP server come later.

## Pipeline runner

Each project keeps its steps in `pipeline.json` (new projects start from the Floating diorama template: Build, Bake light, Export GLB, Compress, Capture posters, Ship). A step has an `id`, a `kind` (`blender`, `shell`, `gltf-transform`, `toktx`, `ffmpeg`), a script or command, an args array, `inputs` and `outputs` globs and `depends_on`.

- Programs are spawned directly with an args array, never through a shell, on the same PATH the toolchain check used. On Windows they start with `CREATE_NO_WINDOW` inside a job object, so Cancel ends the whole process tree.
- Blender always runs in the background: a `blender` step without `-b` is refused. The only time Maquette opens Blender's window is when you click Open in Blender.
- A step is skipped when its input files, script, args and tool version hash to the same key as the last good run (kept under `cache/`). Run from here starts at a step; Ignore the cache reruns everything.
- One run at a time per project; more runs queue. Runs and their logs (last 5) live in the project's cache folder.
- The Compress step reads the per model presets (texture size, ETC1S or UASTC, meshopt level). A locked row keeps its values and says why.

### Starter scripts

A new project gets small, commented scripts in its `tools/` folder when its pipeline is first written, so it runs end to end on day one. They read their paths from args and are the project's to edit; Maquette never writes over a file that exists.

- `build_scene.py`: opens `blender/scene.blend` when there is one (else an empty scene with a sun), imports each `models/*.glb` into its own collection under "Models", lays overlapping models out side by side and saves `blender/scene_built.blend`.
- `bake_light.py`: a stub that bakes nothing yet. It saves `blender/scene_baked.blend` and writes `blender/bake/bake.json` saying so.
- `export_web.py`: one GLB per model collection (and per other top level object) into `web/raw/`, named in snake case, uncompressed for the Compress step.
- `render_stops.py`: one poster per camera stop, a camera with the `maquette_stop` custom property (its number sets the order) or a name starting with `stop_`. It always writes `web/posters/stops.json`, empty when there are no stops.
- `ship.mjs`: copies `web/dist/*.glb` to `public/models` and the posters to `public/posters` in the project's site repo. Without a site repo it says so and passes.

Character showcase projects get `export_character.py`, `render_stops.py` and `ship.mjs`; their pipeline starts from `blender/character.blend`, which you add.

## Model inbox

Maquette watches the workspace `inbox/` folder for `.glb` and `.gltf` files. Each file is read in Rust: triangles, meshes, materials, textures and their sizes, skins and joints, animations. Checks compare that with the budgets: triangles, texture size, a Mixamo rig for characters, PBR maps, separate objects merged into one mesh, and hair or tail vertices weighted to finger bones. Accept to Blender moves the file into the project's `models/` folder as a new asset version.

Two checks are heuristics:

- **Merged objects:** a mesh whose welded triangles form several islands far apart (gaps over 10% of the mesh size, each island at least 1% of it) probably holds objects that were separate. Scenes are exempt.
- **Weighted to finger bones:** a vertex whose strongest joint is a finger bone, but which sits more than 15% of the model height away from that bone (or above the head joint), is counted. Draco or meshopt compressed geometry skips both checks.

The preview stands every model in the same studio: a graphite backdrop, a soft floor with a contact shadow, a warm key, a warm fill and a cool rim light, and a three quarter camera framed to the bounds. Shaded, wireframe over clay, a numbered UV checker, and the skeleton drawn in clay over a ghost of the mesh. It reads meshopt geometry and KTX2 textures (the Basis transcoder ships in `apps/web/public/basis`).

### Thumbnails

Inbox models get a 256 px thumbnail when they arrive, and Board assets get one on Accept and for each new version. A headless Blender job (always `blender -b`, never the GUI) renders the model in the studio above, on the runner's own `thumbnails` lane, so renders queue and never block the UI. Renders are cached by content hash in `cache/thumbs/` and copied to `<project>/thumbs/<asset>_v<n>.png`. When Blender is missing, or cannot read the file (KTX2 textures), the UI draws the model once offscreen with three.js in the same studio and stores that instead. While a render runs, cards and rows show the asset kind's icon.

Render thumbnails by hand with `cargo run -p maquette-core --example render_thumbnail -- <thumbs dir> <model>...`.

## Stack

- **Shell:** Tauri 2 (`apps/desktop`), macOS and Windows
- **UI:** React 19, Vite, Tailwind CSS v4, zustand (`apps/web`)
- **Kit:** typed components with a theme generated from `tokens.json`, Phosphor icons, Geist fonts bundled (`packages/ui`)
- **Core:** Rust domain logic (`crates/maquette-core`): tool detection and `--version` parsing, the `maquette.json` workspace config, projects, assets, the pipeline runner and cache, GLB analysis and the inbox watcher
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

The mock runner streams Blender style output and fails Capture posters once. The mock inbox lists a character, two props and a broken download. Its previews load real meshopt and KTX2 models, and the Board and Inbox show real studio thumbnails, all from `apps/web/src/assets/mock` (see the README there: that artwork is reserved and not covered by the repository license). For hand testing a real inbox, `cargo run -p maquette-core --example write_fixtures -- <dir>` writes small generated GLBs.

### End to end check

`cargo run -p maquette-core --example new_project -- <workspace dir> <name> [--site <repo>]` creates a workspace (if needed) and a project with its pipeline and starter scripts, the way the app does. `cargo run -p maquette-core --example run_pipeline -- <project dir>` then runs a real project's pipeline from the terminal with the same runner the app uses, fills `{workspace}` and `{siteRepo}` like the app, and prints each step's state. A second run with nothing changed reports every step cached except Ship, which writes outside the project and always runs.

## Scripts

| Script              | What it does                                                                                                                    |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `pnpm lint`         | ESLint over the workspace                                                                                                       |
| `pnpm typecheck`    | TypeScript in every package                                                                                                     |
| `pnpm test`         | Vitest for `apps/web` and `packages/ui`                                                                                         |
| `pnpm format:check` | Prettier check (`pnpm format` writes)                                                                                           |
| `pnpm rust:fmt`     | `cargo fmt --check`                                                                                                             |
| `pnpm rust:lint`    | `cargo clippy` with warnings as errors                                                                                          |
| `pnpm rust:test`    | `cargo test` for the workspace, which also regenerates the IPC bindings                                                         |
| `pnpm gate`         | All of the above, in order                                                                                                      |
| `pnpm build`        | Release build of the desktop app                                                                                                |
| `pnpm visual`       | Screenshots of mock mode in headless Chromium (set `PLAYWRIGHT` to a local Playwright `index.mjs` and `OUT_DIR` for the output) |

Commits follow Conventional Commits with a required scope (`desktop`, `web`, `ui`, `core`, `workspace`, `deps`, `docs`, `scripts`).

## License

All rights reserved. See [LICENSE](LICENSE).
