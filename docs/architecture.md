# Architecture

How the code is laid out and what the linters hold it to. The rules live in `eslint.config.mjs`,
`scripts/lint-meta.mjs`, `Cargo.toml` (`[workspace.lints]`), `clippy.toml` and `rustfmt.toml`; this
page says why and how to work with them.

## Workspace

| Path                     | What it is                                                              |
| ------------------------ | ----------------------------------------------------------------------- |
| `apps/web`               | The React UI, served by the Tauri shell or by Vite in mock mode         |
| `apps/site`              | The landing page: Astro with a react-three-fiber island in `src/scene`  |
| `apps/desktop/src-tauri` | The Tauri shell: a thin composition root that wires the core into Tauri |
| `packages/ui`            | The UI kit: typed primitives and the theme                              |
| `crates/maquette-core`   | The Rust domain logic                                                   |

Dependencies run one way: `packages/ui` -> `apps/web`, and `maquette-core` -> `src-tauri`. The web
app reaches the backend only through `src/ipc` (generated tauri-specta bindings, or the in-memory mock).

## apps/web

```
src/
  App.tsx, main.tsx     the app root
  features/<feature>/   one folder per feature, one folder per component inside it
  state/                zustand stores (use-*-store.ts) and the pure logic they run
  ipc/                  the typed client, file URLs and the mock backend
  lib/                  small shared helpers (clock, storage, format, hooks)
  test/                 test setup and the mock IPC helpers
```

### Folder per component

Every component is a folder named after it:

```
features/board/BoardView/
  BoardView.tsx         the component (default export), rendering only
  BoardView.hooks.ts    its state, effects, refs, memos and store selectors
  BoardView.types.ts    optional: props and view types
  BoardView.test.tsx    required: a colocated test
  index.ts              barrel: export { default as BoardView } from './BoardView'
```

- One component per folder. A sub component used by one parent may sit in a nested folder.
- The `.tsx` holds no stateful hooks: it calls its `use<Name>` hook and renders. The hook file
  exports at most four hooks.
- Pure helpers and constants are plain `.ts` modules named after their main export, with a `.test.ts`.
- Other code imports the folder (`@/features/board/BoardView`), never a file inside it.

### Feature boundaries

A leaf feature imports `@/state`, `@/ipc`, `@/lib`, `@maquette/ui` and the shared features, never
another leaf feature. When a leaf needs another feature's UI, a composition root passes it in (the
shell hands the 3D queue to the Board as the `queue` render prop).

| Kind              | Features                          | May import other features     |
| ----------------- | --------------------------------- | ----------------------------- |
| Shared            | `common`, `config`, `viewer`      | No (everyone may import them) |
| Composition roots | `shell`, `onboarding`, `settings` | Yes: they assemble the views  |
| Leaves            | every other feature               | Only the shared ones          |

The lists are `SHARED_FEATURES` and `COMPOSITION_ROOTS` in `eslint.config.mjs`.

### packages/ui

The kit stays flat: `src/components/<group>/<Name>.tsx`, exported from `src/index.ts`. A group is a
shelf (shell, status, inputs), not a feature, so primitives may use each other. Component state goes in
a `<Name>.hooks.ts` beside the component, like in the app.

## Lint tiers

Every rule is `error` or `off`, never `warn`; `pnpm lint:meta` fails if any rule resolves to `warn`,
including from a preset.

1. **Base:** `@eslint/js` and `typescript-eslint` recommended, React and React Hooks (exhaustive deps
   at error), Prettier for formatting.
2. **House presets** for the `.ts` and `.tsx` of `apps/web`, `apps/site` and `packages/ui`, each
   spread whole so new preset rules arrive with a version bump: `@noctcore/eslint-plugin-code-quality`, `-async-safety`, `-contracts`,
   `-security`, `-observability`, `-react`, `-architecture` and `-monorepo` (the `@maquette` scope).
3. **Project options:** the feature root, shared features and composition roots above, the `@/`
   alias for deep imports, and the trimmed sibling set (test and barrel required; hooks and types
   when needed, since there is no Storybook).
4. **Scoped overrides:** a handful of `files: [...]` blocks, each with a one line reason. Inline
   `eslint-disable` and `@ts-ignore` are not allowed (lint-meta checks it).

`pnpm lint:meta` adds the whole-repo checks: every TypeScript module in `apps/web/src`,
`apps/site/src` and `packages/ui/src` at or under 400 lines (tests excluded), every Rust module at or
under 400 code lines, no inline suppressions, and no `warn` severities. Over-cap files can be frozen in
`scripts/lint-meta/baselines/` while they are split; a frozen entry may only shrink, and one that is
no longer needed fails until it is removed.

## Rust lint policy

`[workspace.lints]` in the root `Cargo.toml`, inherited by every crate with `[lints] workspace = true`:

- `unsafe_code = "deny"`
- `rust_2018_idioms` and `unreachable_pub` (items only used inside the crate are `pub(crate)`)
- clippy: `await_holding_lock`, `unwrap_used` (propagate errors; tests use `expect` with a message),
  `todo`, `dbg_macro`

The gate runs clippy with `-D warnings`, so these are hard failures. `clippy.toml` sets
`cognitive-complexity-threshold = 20` and `too-many-arguments-threshold = 8`; past either, extract a
function or group the arguments. `rust-toolchain.toml` pins the toolchain the repo builds with and
`rust-version` in the workspace states it. A module over 400 code lines is split into submodules,
with its tests in a sibling `tests.rs`.

## MCP tools

The MCP server lives in `apps/desktop/src-tauri/src/mcp`: `server.rs` serves Streamable HTTP on
127.0.0.1 behind the bearer token, `tools.rs` holds the server instructions and joins one
`#[tool_router]` per file in `tools/` (`concepts.rs` starts and reads the Lab, `picks.rs` stars and
picks concepts onto the Board, `sending.rs` sends to 3D, and so on). A tool does what the matching
UI action does by calling the same `maquette-core` function or the shared command helper in
`commands/` (`pick_concepts` and Send picks to Refs both end in `concepts::picks::place_at_refs`),
so the checks are the same. After a change it tells the windows through `Host` (`project_changed`,
`concepts_changed`, `layout_changed` and the rest), which emits the same events the UI listens to.
The integration tests in `apps/desktop/src-tauri/tests/` run a real client against a test `Host`;
a new tool goes into the tool list in `mcp_handshake.rs`, the instructions and the README.

`tools/built.rs` adds, rebuilds and lists built assets through `commands/built.rs`, the same
helpers the Board's commands use. Builds go through one `maquette_core::built::Builder` for the
whole app, so a build an agent starts and one from the Board queue behind each other, and
`run_pipeline` without `from` calls the same `before_run` as the Run button.

`tools/motion.rs` holds `get_motion`, and the `states` and `clips` edits `set_layout` takes
(`projects.rs` turns them into a layout patch, the same `apply` the Layout editor's save uses).
`tools/timeline.rs` holds the `explode` edits `set_layout` takes, each asset's whole explode track
or its removal; the Timeline's stop and scroll changes go through the `states` edits.
`tools/paths.rs` holds the `paths` edits (add, change or remove a path, anchor its ends) and the
`energize` edits (a path's whole energize track or its removal), which `projects.rs` turns into the
layout patch's `paths` and `pathTracks`.

`tools/library.rs` holds `list_library`, `add_to_library`, `use_from_library` and
`update_from_library`, which call the same `maquette_core::library` functions as the commands and
emit `libraryChanged` and the project events. `tools/animations.rs` holds `list_animations`,
`import_animations` and `save_model_clips`; import paths must resolve inside the workspace.

`tools/trash.rs` holds `remove_assets`, `list_removed_assets` and `restore_asset`, which go
through `commands/trash.rs` like the Board's Remove from project and Recently removed.

`tools/page.rs` reads, starts and stops the site dev server the Page view shows
(`get_page_preview`, `start_page_preview`, `stop_page_preview`) through `commands/page.rs`, so an
agent and the view share the one server and its state.

`site/preview` keeps that one server. A plan carries `port_in_args` (the command names `{port}`) and
`key` (the command line before the port is filled in). The address shown is the configured `url`,
else the one the server prints, else the expected port, but that last only when the port is in the
args or after a grace of 8 s, since a server given only `PORT` may ignore it. A server running at a
guessed address that later prints another one moves to it, and a changed `key` starts afresh. On the
web side `use-page-store` holds the server for every view: `features/page/ServerControls` (the
frame's Stop), `features/pipeline/PreviewSiteButton` and `features/shell/SiteServerItem` (the status
bar menu) all stop it through `stopServer`, with `common/StopServerButton` shared.

## Game projects

`ProjectManifest.target` is `website` or `game`; a game keeps `GameSettings` in `game` (boxed, so
the events that carry a manifest stay small). `maquette-core/src/game/` holds the rules: `detect.rs`
reads a game repo, `scan.rs` and `import.rs` turn its models into assets (version 1 is the repo
file, recorded in `game.models`), `budget.rs` checks models against their folder's budget,
`index.rs` writes `web/game-index.json` before every run (each model's target path, whether it is
still the untouched import, its `textureCap` from its folder budget with the global cap as the
fallback, the export settings), `ship.rs` plans what Ship would change against
`web/ship-record.json` (a replaced file whose game copy is still the imported one is `imported`;
`detect::uncommitted` counts `git --no-optional-locks status --porcelain`, read only), and `settings.rs` saves a settings change and re-ensures the pipeline. Budget fix-ups never give a
rigged model (`BudgetCheck.rigged`, any joints) Decimate, only Shrink textures. The
game template is `pipeline/template/game.rs` with its starters in `starter/` (`export_game.py`,
`game_export.mjs`, `ship_game.mjs`).

The desktop side is `commands/game.rs` (Tauri commands, `before_run` and `ship_gate`) and
`mcp/tools/game.rs`. Import asks the thumbnail service for each new model through
`commands/thumbs.rs::render_missing` (`thumbs::missing` lists the models with no render), and
`thumbnail_render_missing` does the same for a whole project. A game's Ship is gated in every run: `ship_gate` returns a `ShipGate` with
`ShipReason::Game`, so the run started from the Run button waits in the same approvals list as an
agent's. In the web app, `state/game.ts` mirrors the core's rules (stage names, file and budget per
asset, Ship plan groups), `state/views.ts` hides the website only views, and the game UI lives in
`features/game` (Import models), `features/inspector/BudgetPanel` and
`features/pipeline/{GameExportPanel,ShipView}`. `stageViews` takes the target, so every top bar
names the game stages, and `SceneBudgetMeter` turns into `ModelBudgetMeter` in a game. Motion in a
game shows clips only, with no scene budget or ship switches.

## Motion

`maquette_core::motion` reads what moves. `motion.rs` builds the view for a project: every model in
Build scene order with its rig (`rig.rs`: joints, kind, the humanoid bone map and most influences
per vertex), its clips (`clips.rs` reads each glTF animation's keys from the GLB: frames, key rate,
channels, keyframe bytes, root travel and looping; keys packed by meshopt are counted but not read,
so travel and looping come back unknown) and its parts (the nodes that draw a mesh). Clip bytes are
weighed in the file the model is weighed by, so `budget` is the scene budget with the motion share
in it and in the first view; motion has no budget of its own. `save.rs` checks the parts a state
names against the models and writes the states and clip choices with `layout::apply`.

Object states and clip choices live in `layout.json` (`layout/states.rs`): `states` keyed by asset
id (states, transitions, triggers) and `clips` keyed by asset then clip name (`{ "ship": false }`,
a clip with no entry ships). `layout::validate` checks them with the rest of the layout: at most 16
states and 32 triggers, transitions and triggers that name states and stops that are there, poses
within reach. Older layouts load with both empty. The starter `build_scene.py` removes the clips
kept off before Export GLB writes the animations; `clips_off_supported` is false for a project
whose copy has no such step, and the web side says the clips all ship.

The web side is `state/use-motion-store.ts` (the draft and the player), `state/motion.ts` (budget,
timeline and list rules) and `state/motion-states.ts` (state edits, easing and posing), the
components in `features/motion`, and the canvas in `features/viewer/MotionStage` with
`viewer/motion-pose.ts` (clip seek, part offsets, the root trail). The shell reloads the store on
`layout_changed` and `prepare_changed`.

## Timeline

The Timeline view edits two things in `layout.json`. Its state changes are the props' stop and
scroll triggers in `states`, the same ones Motion lists, so there is one copy of them. Its explode
tracks live under `timeline.explode` (`layout/timeline.rs`): per asset a `distance` in metres, an
`easing` and at most one `{ stop, amount }` key per camera stop, amount 0 to 1. `layout::validate`
checks them with the rest of the layout, `layout::prune_stops` drops the keys and stop triggers of
a removed stop, and older layouts load with no tracks.

`maquette_core::timeline` builds the view (the stops, every model with its parts from
`motion::parts`, the states and the tracks) and saves it: `with_timeline` replaces only the stop and
scroll triggers of the assets the view names, keeps their click and hover triggers, and refuses an
asset without states. `commands/timeline.rs` reads and saves it, and the Layout editor's save keeps
the timeline as it is.

The web side is `state/use-timeline-store.ts` (the draft, saved half a second after each edit),
`state/timeline.ts` (positions along the page, marks, edits) and `state/timeline-checks.ts`, the
components in `features/timeline`, and `features/page/PagePreview`, which the shell passes in as
the preview. The playhead is the Page view's: a seek goes to the page through the page bridge and
the page's answers move it. The shell reloads the store on `layout_changed` and `prepare_changed`.

The site plays it: `apps/site/src/scene/timeline.ts` turns tour progress into each prop's pose
(scroll ranges scrubbed, stop changes timed by their transition) and each track's amount, and
`scene/tracks.ts` moves the built model's nodes, found by their names in the file, from their rest
transforms. A part spreads straight out from the model's centre.

## Paths

Paths are curves through the scene that the site draws as tubes and lights as the page scrolls.
They live in `layout.json` (`layout/paths.rs`): `paths` is a list of `{ id, name, curve, points,
start, end, width, color, glow }`, points in the layout's space (metres, y up), `curve` either
`polyline` or `catmull-rom`, and `start` and `end` the asset ids the ends follow. Their energize
tracks live under `timeline.paths`, keyed by path id, with an `easing` and at most one
`{ stop, amount }` key per camera stop, amount 0 to 1, checked by the same `check_keys` as the
explode tracks. `layout::validate` checks both (at most 32 paths of 2 to 128 points, widths from
1 mm to 1 m, `#rrggbb` colours, tracks of paths that are there), `layout::prune_stops` drops the
keys of a removed stop, and older layouts load with none.

An anchored end is stored where it is, not as an offset. `paths::follow` moves it by as much as its
model's placement position moved, and runs wherever placements change: in `layout::apply` (the
patches agents and Motion send) and in `commands/layout.rs`, whose save keeps the paths the file
has and follows them. Only the position follows; a turn or a scale does not. `paths::with_paths`
replaces every path and track at once for the editor (`commands/paths.rs`, `paths_save`), and the
editor reads them with the rest of the layout (`layout_get`).

The web side is `state/use-paths-store.ts` (the draft, saved half a second after each edit, and
the selection), `state/paths.ts` (point, anchor and look edits, lengths, the tube's triangles and
what a path adds to `layout.json`), the Inspector's Paths tab in `features/inspector` (`PathsEditor`,
`PointsTable`, `PathsPanel` and `PathsCanvas`, which reuses the Layout editor's model loading and
placement and three's transform gizmo on a point handle) and the Timeline's `PathLane` and
`PathKeyEditor`, which edit `timeline.paths` through the Timeline store. The shell reloads the store
on `layout_changed`, and the editor loads the layout again when it opens.

The site draws them: `fromLayout` lifts the points with the rest of the layout, `scene/path-mesh.ts`
builds each path as one `TubeGeometry` (8 sides, a ring about every two widths, 16 to 256 rings)
with a shader that lights it from its first point by a `uAmount` uniform, and `scene/PathTubes.tsx`
sets that uniform each frame from `energizeAt` (`scene/paths.ts`), the track's amount eased between
its keyed stops by `keyedAt`. One draw call per path. Build scene and the posters do not draw them.

## Library

`maquette_core::library` owns it (`mod.rs`, `files.rs`, `source.rs`, `add.rs`, `use_in.rs`,
`update.rs`, `view.rs`). It lives in `<workspace root>/library/`, outside every project. An entry
is `library/<entry-id>/` with `entry.json` (name, kind, tags, refs, versions), `models/` (one
`<key>_v<n>.glb` per version, or a folder for an `.obj` with its files), `thumbs/` and `refs/`. A
version records the model file, stats, source, credits when known, a sha256 hash, the project,
asset and version it came from, and a snapshot of the asset's prep (real size and fix-ups) and
material tweaks.

- **Origin.** The library copies and never links. A copied asset records `origin: { entry, version }`.
  Uses are derived from the project assets' origins when the view is built, so they never go stale.
- **Add.** The newest model of an asset is copied with its picture, refs and prep. The same hash adds
  nothing; a newer model of an asset already in the entry adds the next version.
- **Use.** A version is copied into the project as a new asset at v1 (Blender stage, done). A
  website project refuses an `.obj`; a game project takes either and the next Game export writes
  the game's format. No credits are spent.
- **Update.** `update.rs` builds a plan: what carries over, and what is dropped with a reason
  (Decimate when the new model has that many triangles or fewer, Shrink textures when no texture is
  larger than the limit, tweaks for a material name the model lost). Applying it adds the library
  version as the asset's next version.
- **Path safety.** An entry id is a single path segment, and every recorded path must stay inside the
  entry folder. A write lock serialises library writes.
- **Edges.** The Tauri commands `library_get`, `library_add`, `library_use`, `library_update_plan`
  and `library_update` are in `apps/desktop/src-tauri/src/commands/library.rs`. They emit
  `libraryChanged`. The MCP tools are in `mcp/tools/library.rs`. The web state is
  `apps/web/src/state/library.ts` and `use-library-store.ts`, and the view is
  `apps/web/src/features/library`.

## Animation library

`maquette_core::animations` keeps clips once in `<workspace root>/library/animations/`, skeleton
only. A clip is `animations/<clip-id>/` with `clip.json` (name, pack, tags, the source rig with its
kind, bones, rest pose and hips height, frames, rate, length, root motion, loop, size, origin, the
sha256 of its keys and a revision) and `clip.glb` (bones and one animation, no mesh), plus
`source.<ext>` only when the person keeps the original.

- **Reading.** `read_clips.py` runs headless on the runner's `animations` lane in a scratch folder
  under the workspace cache. It imports each FBX or glTF, keeps the armature with the most bones,
  drops every mesh, measures each action (frames, rate, root travel and direction, loop) and writes
  a skeleton only GLB per clip with a report. A lone FBX take is named after its file.
- **Import.** `import.rs` stages files (a zip is unpacked, `__MACOSX` skipped), reads them, and
  `import/classify.rs` sorts each clip: new, a duplicate (the same key hash in the library or an
  earlier file) or a revision (the same pack and name with other keys), and files with no keys left
  out. The plan's token names the staged folder; `import/commit.rs` writes what the person chose.
- **Rig.** `rig.rs` reads the source rig from the bone names: Mixamo through the humanoid map,
  Tripo through its own bone table or a Tripo send, Custom otherwise.
- **Save.** `save.rs` reads a project asset's newest model or an Inbox file without Blender for the
  plan, then headless for the chosen clips; clips that came from the library are never saved back
  and known hashes are skipped.
- **Retarget.** `retarget/` puts clips on a rigged model. `bones.rs` maps both skeletons onto the
  humanoid bones of Motion's rig check (Tripo's names filled in), so one map per model covers every
  source rig; `settings.rs` normalises per model clip settings; `bake.rs` copies the model and
  clips into a scratch folder and runs `retarget_clips.py` headless, which turns the model's limbs
  to the clip's rest pose, applies each clip bone's world rotation change to its mapped bone, scales
  the hips by height and measures each clip (travel, how far the hips go, planted foot slide);
  `review.rs` grades them. `bake` adds the draft as the next version through the build checks.
  `uses.rs` keeps the project's `animations.json` (saved map, baked clips with revision and
  settings, last review), which gives the library's uses and update offers.
- **Edges.** The Tauri commands are in `commands/animations.rs` and emit `libraryChanged` (and
  `projectChanged` after a bake); the MCP tools are `mcp/tools/animations.rs` and
  `mcp/tools/retarget.rs`; the web state is `state/animations.ts`, `state/retarget.ts` and their
  stores, the views `features/animations` and `features/retarget`.

## Remove and restore

`maquette_core::assets::remove` takes assets off a project without deleting anything.

- **Refusals.** A run going in the project refuses the whole call. An asset that is building, or
  whose Tripo or Meshy send is running, is refused alone; sends waiting for approval, queued or
  waiting for a By hand model are stopped first.
- **Files.** `files.rs` collects the asset's own files (model versions, an `.obj` version's folder,
  thumbnails, refs no other asset or send uses, built script and helpers, handoff folders) and moves
  them to `<workspace>/trash/<project>/<asset>-<time>/files/`, rolling back when one fails.
- **Record.** `trash.rs` writes `removed.json` beside them: the asset entry, the moved paths, the
  stopped sends and what `links.rs` took out of `layout.json` (placement, stop membership, path
  ends, states, clips, explode track), `prepare.json`, `materials.json`, `built.json` and the game
  model. The library entry stays.
- **Restore.** `restore.rs` moves the files back, re-adds the asset and puts back each link where
  nothing has taken its place. Stopped sends stay stopped.
- **Edges.** The Tauri commands `asset_remove`, `asset_trash_list` and `asset_restore` are in
  `commands/trash.rs`; the MCP tools are in `mcp/tools/trash.rs`. The web state is
  `apps/web/src/state/trash.ts` and `use-trash-store.ts`.

## Built assets

`maquette_core::built` owns them. `add.rs` copies the script and helpers into `<project>/built/`
and records the source in `built.json` (keyed by asset id, refused when written by a newer
Maquette). `mod.rs` computes the build key and each asset's state (new, current, stale, failed,
missing, building). `blender.rs` stages the files in a scratch folder under the workspace cache
and runs `build_asset.py` (embedded in the crate) as a one step pipeline on the runner's `built`
lane, the way the fix-up previews run. `build.rs` turns the GLB into the next version, runs the
Inbox checks for warnings and keeps the last build on the record. The desktop side adds the
thumbnail, the window events (`BuiltChanged`, `ProjectChanged`) and the pre run rebuild; the web
side is `state/use-built-store.ts`, the Board card and `features/board/AddBuiltDialog`.

## Website ship settings

`maquette_core::ship` owns `ship.json`: the style (scene or models), the texture format (KTX2,
WebP or AVIF), the quality and each asset's choice (auto, never, always) and file name. A missing
file means the old behaviour: scene style, KTX2, versioned names. The run setup turns them into
the `{exportOrigin}` var (Export GLB's `--origin`), the texture format Compress uses
(`presets/image.rs` for WebP and AVIF, with `materials/split.rs` marking the smaller data maps by
name so one `resize --pattern` pass reaches them) and the ship index's `choice`, `origin` and
`sizeGuard`, which `ship.mjs` reads to hold files back. `ship/plan.rs` mirrors those rules for the
dry-run plan, and `ship/report.rs` builds the size report. `site::save_target` writes the folders
into `maquette.site.json`, keeping its other keys. `build_scene.py` stores each collection's move
in `maquette_moved` so `export_web.py` can undo it. A build only run marks the ship steps skipped
in `graph::plan_run`. The desktop side is `commands/ship.rs` and `mcp/tools/ship.rs`; the web
side is `state/use-ship-store.ts` and `features/pipeline/SiteShipView`.

## Baked light

The Bake light step and what it hands the site, end to end:

- **Settings.** `maquette_core::pipeline::bake` reads and writes the Bake light step's `--size`,
  `--samples` and `--denoise` / `--no-denoise` args in `pipeline.json`; there is no second copy,
  so the cache reruns the bake when they change. Only a step running `tools/bake_light.py` has
  settings. An older or edited copy of that script cannot denoise, so the flag is only written
  when the script is the current starter or the args already carry it. The desktop side is
  `commands/pipeline/bake.rs` (`pipeline_save_bake`, and `bake` on the pipeline view) and the
  MCP tools in `mcp/tools/bake.rs` (`get_bake_settings`, `set_bake_settings`, `set_skip_bake`).
  The web side is the Pipeline view's `BakePanel` (`saveBake` in the pipeline store) and the
  Skip the bake switch in `PrepEditor`.
- **Skip bake.** `skipBake` on an asset's `prepare.json` entry (`ModelPrep::skip_bake`, null when
  nobody chose). Build scene tags every object of a skipping asset `maquette_skip_bake`; the bake
  leaves tagged meshes out (they still shadow the others), so they get no lightmap and no
  `lightmap` UV, and the export drops a `lightmap` UV set nothing samples. An Inbox accept sets
  `skipBake: true` when neither the Inbox nor the saved entry chose
  (`prepare::SKIP_BAKE_FROM_INBOX`): Inbox models are provider scans, whose tiny UV islands bake
  into flat lit facets. Built assets and models added by hand bake unless set.
- **Glowing materials.** A material with a live emission (a Principled BSDF whose emission
  strength and colour are both non zero or linked) or an unlit emission shader is never given
  a lightmap: the bake skips a mesh whose every material glows, and the export leaves such a
  material's emission alone. `glows()` is the same function in `bake_light.py` and
  `export_web.py`, and a test holds them together.
- **The contract with the site.** A baked material ships its lightmap as the glTF emissive
  texture, sampled with `TEXCOORD_1`, and the material extras
  `{ "lightmap": <scale>, "lightmapUv": 1 }`. The map stores light divided by the scale, sRGB
  encoded. A page moves it to a light map: in three.js `lightMap = emissiveMap`,
  `lightMapIntensity = scale * PI`, then clear `emissiveMap` and set `emissive` to black. A
  material without the extras has no bake and is lit live; its emission is its own.

## Page view

The Page view shows the project's site, or an HTML page concept, with the models live in it.

- **Settings.** `maquette_core::page` keeps `page.json` in the project: the source (`site` or
  `concept`), the path on the site, the concept file (an `.html` inside the workspace) and the
  project's own dev command. `commands/page.rs` adds `page_get`, `page_save`,
  `page_server_start`, `page_server_stop`, `page_server_release` and `page_open_in_browser`.
- **The dev command.** `maquette_core::site::preview::command` picks it: the project's dev
  command, then the `preview` of `maquette.site.json`, then the first of the `dev`, `start` and
  `serve` scripts in `package.json`, run by the manager of the nearest lockfile. A command is a
  program and its args, never a shell line; a typed one is split into words and passed as it
  is. `{port}` in an arg is the free port Maquette picks, a `package.json` command gets
  `--port {port}` (`-- --port` for npm) unless the site config names its address, and `PORT`
  is set too.
- **The server.** `site::preview::server` runs one dev server for one site at a time, in its
  own process group (a job object on Windows) under a small `sh` watchdog on macOS and Linux,
  so a stop or the app going away, even by a crash, ends the dev tool and its children. A start
  returns at once; a watcher marks it running once the address answers, failed when it exits
  or after 90 seconds, and keeps the last 200 log lines. A server already answering at the
  site's address is reused and never killed. Each start records an owner (the Page view, an
  agent, or Preview on site); the view lets go of its server 400 ms after it closes (so a
  remount keeps it) and only stops it when it is still the owner, so Preview on site and
  agents keep theirs. Every change emits `PageServerChanged`, which the view, the status bar
  and `use-page-store.ts` follow.
- **The frame proxy.** `site::preview::frame::FrameProxy`: once the server answers (started or
  reused), `SitePreview` starts one in front of it on `127.0.0.1` and puts its address in the
  status's `frame_url`; a stop, a failure or another start drops it, which closes every
  connection. The next one tries the last port first, so the page keeps its storage. It runs
  hyper on its own two thread tokio runtime (as the MCP server does) and opens one loopback
  connection to the dev server per request. It forwards only to that loopback dev server, never
  elsewhere: a target that is not plain `http` on this machine is refused (the frame then loads
  `url` directly), the request's own target is cut to its path and query, and only loopback
  addresses are dialled. `frame/headers.rs` holds what changes: on the way in the hop by hop
  headers go, `Host` is the dev server's and an `Origin` or `Referer` of the proxy reads as the
  dev server's (Next server actions and dev origin checks compare them); on the way out the hop
  by hop headers and `X-Frame-Options` go, `frame-ancestors` leaves `Content-Security-Policy`
  and `-Report-Only` (a policy without it is left byte for byte, one with nothing else goes),
  and a `Location` on the dev server points at the proxy. Bodies stream through untouched, and
  a `101` splices the two upgraded connections (Next, Vite and Astro HMR). The one exception is
  a page load (`frame/inject.rs`): a `GET` whose `Sec-Fetch-Dest` is `document` or `iframe` (or,
  without one, that accepts `text/html`) goes without `Accept-Encoding`, and a `text/html`
  answer (decoded if it still comes gzip or deflate; brotli passes as it came) gets
  `<script src="/__maquette/bridge.js" defer>` after `<head>` (else before `</head>`, else after
  `<body>`, else nowhere within 256 KB). Only the bytes up to that spot are held back, so the
  page still streams; its `Content-Length` and `Content-Encoding` go and it is sent chunked.
  The proxy answers everything under `/__maquette/` itself and never forwards it: there it
  serves `frame/shim.js`, the bridge shim, from its own origin, so a site CSP with
  `script-src 'self'` runs it and no directive is loosened (a nonce or `'strict-dynamic'`
  policy still blocks it). The address bar, the status bar and `page_open_in_browser` keep
  using `url`. The `frame_proxy` example runs one for the visual check.
- **The frame.** The page loads in an iframe (`features/page/PageFrame`), laid out at the
  device's CSS width and scaled down to fit, with `sandbox` that allows scripts but no top
  navigation or popups and no referrer. An iframe rather than a second native webview: Tauri 2
  child webviews need its `unstable` feature, sit above the DOM so they cannot be scaled, tiled
  or overlaid, and break the compare board. The frame loads the server's `frameUrl` (the frame
  proxy), else its `url`. The CSP's `frame-src` allows `localhost`, `127.0.0.1` (the proxy) and
  the asset protocol (for a page concept) and nothing else.
- **The page bridge.** Window messages between the app and the page, all with `maquette: 1`.
  The app says `{ type: "hello" }` once the frame loads (again every 500 ms until the page
  answers) and `{ type: "goto", stop, smooth }` as the scrubber moves (`stop` is fractional);
  the page answers `{ type: "ready", stops: [{ name }], features }`, then `{ type: "at", stop,
scroll: { top, max } }` as it scrolls and `{ type: "stats", frameMs, p95Ms, drawCalls? }`
  about once a second. The page answers the origin that said hello; the app hears only the
  frame's own window and posts to `*`, so neither side depends on the page's origin, which is
  the proxy's. `?maquette-poster=1` asks the page for its no WebGL version.
  `apps/site/src/scene/bridge.ts` is the reference page side.
- **The bridge shim.** `frame/shim.js` speaks bridge v1 for a page without a bridge. Hello
  carries the project's camera stops, which the shim says back in `ready`; every message it
  sends has `source: "shim"`. `goto` scrolls to the element whose id is the stop's slug or
  whose `data-maquette-stop` is its name, slug or number, else to an even spread over the
  scroll range (stop i of n at i/(n-1)); `at` maps the scroll back; `stats` are mean and p95
  of requestAnimationFrame deltas over 120 frames, without draw calls (the readouts keep their
  estimate). It answers only `window.parent`, 300 ms after `load`, and lives across client
  side navigation. It stands down for good once `window.__maquetteBridge` is set, which the
  site's own bridge does. The app keeps saying hello after a shim `ready`, takes the first
  `ready` without `source: "shim"` over it (`use-page-store.ts` then ignores the shim for that
  frame), and the scrubber hint and readouts say which one answered.
- **Numbers.** Readouts come from the page (frame time, draw calls) and from the inspect data:
  the `site` source is the current ship, the `shipped` source this run. Device budgets
  (`DEVICE_BUDGETS` in `state/page-numbers.ts`) give the phone, laptop and desktop verdicts.

## About and updates

Settings > About reads `app_about` (`commands/about.rs`): the version, the commit and build date
`build.rs` stamps into a release build ("dev" otherwise), the OS and its version, the arch, the
Tauri version and the Blender version of the last toolchain check. Its links open through
`app_open_link`, which only opens the repo and docs addresses in `opener::PROJECT_LINKS`. The copy
block and the prefilled bug report address are built in `apps/web/src/state/about.ts`; nothing is
sent anywhere.

The updater (`src/updater/`) drives `tauri-plugin-updater` from Rust. `updater::mode` decides at
launch: no `plugins.updater.pubkey` (today), a debug build or `MAQUETTE_NO_UPDATER=1` keep it off,
and only `on` registers the plugin and schedules the checks (10 s after launch, then every 6 h).
`updater_check`, `updater_download` and `updater_install` hold the found update and the downloaded
bytes between the user's clicks, and every step also goes out as the `UpdaterUpdate` event. A 404
or a platform missing from `latest.json` reads as `pending`, not as an error. Install refuses while
the runner has work (`PipelineRunner::any_active`) or the web view holds unapplied changes, and
runs the same `shutdown` as an exit before the restart. The web side is `state/updater.ts` (the
state machine), `state/use-updater-store.ts`, `features/shell/UpdatePill` and
`features/settings/AboutSection`. Release signing is in `docs/publishing.md`.

## Adding a feature

1. Create `apps/web/src/features/<feature>/` with one folder per component, each with its test and
   `index.ts`, and put state in the `.hooks.ts` beside it.
2. Import only shared features and non feature modules. If the feature must render another feature,
   let the shell (or another composition root) pass it in. If it becomes something every feature
   needs, add it to `SHARED_FEATURES`; if it assembles other features, to `COMPOSITION_ROOTS`.
3. Give the shell its view (`src/state/views.ts` and `CurrentView` in the shell).
4. Backend work goes in `maquette-core`; the desktop crate only adds the command that calls it.
5. Run `pnpm gate`.
