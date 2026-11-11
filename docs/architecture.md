# Architecture

How the code is laid out and what the linters hold it to. The rules live in `eslint.config.mjs`,
`scripts/lint-meta.mjs`, `Cargo.toml` (`[workspace.lints]`), `clippy.toml` and `rustfmt.toml`; this
page says why and how to work with them.

## Workspace

| Path                        | What it is                                                              |
| --------------------------- | ----------------------------------------------------------------------- |
| `apps/web`                  | The React UI, served by the Tauri shell or by Vite in mock mode         |
| `apps/site`                 | The landing page: Astro with a react-three-fiber island in `src/scene`  |
| `apps/desktop/src-tauri`    | The Tauri shell: a thin composition root that wires the core into Tauri |
| `packages/ui`               | The UI kit: typed primitives and the theme                              |
| `crates/maquette-core`      | The Rust domain logic, and the facade over the crates below it          |
| `crates/maquette-base`      | The bottom layer: errors, files, JSON, the clock, toolchain, run log    |
| `crates/maquette-glb`       | GLB reading without Blender: the inbox analysis, parts, motion and VRM  |
| `crates/maquette-model`     | The stored files: workspace, projects, assets, layout, refs, settings   |
| `crates/maquette-preview`   | The site dev server, the Page view's frame proxy and page settings      |
| `crates/maquette-providers` | The 3D sends: Tripo, Meshy, By hand handoffs, Downloads and the ledger  |

Dependencies run one way: `packages/ui` -> `apps/web`, and `maquette-base` -> `maquette-glb` ->
`maquette-model` -> `maquette-providers`, `maquette-base` -> `maquette-preview`, then all of them -> `maquette-core` ->
`src-tauri`. The web app reaches the backend only through `src/ipc` (generated tauri-specta
bindings, or the in-memory mock).

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
another leaf feature. When a leaf needs another feature's UI, a composition root mounts it (the shell
mounts the Sends panel, and the Board only asks the providers store to open it).

| Kind              | Features                          | May import other features     |
| ----------------- | --------------------------------- | ----------------------------- |
| Shared            | `common`, `config`, `viewer`      | No (everyone may import them) |
| Composition roots | `shell`, `onboarding`, `settings` | Yes: they assemble the views  |
| Leaves            | every other feature               | Only the shared ones          |

The lists are `SHARED_FEATURES` and `COMPOSITION_ROOTS` in `eslint.config.mjs`.

### Waiting

Anything still on its way says one recipe: a verb (`Reading the motion...`), a count when the work
reports one (`3 of 13`) and how long the same wait took last time when that was measured
(`last 12s`, like the pipeline runner's steps). `Loader` and `WaitLine` in `packages/ui` draw it;
`lib/waiting.ts` times a wait with `useWait` and keeps the last run in local storage. A wait that
failed, was left behind or took under 2 s is not a past run. No past run means no time: nothing
guesses one. A run's name holds what makes it different work: `renderWait` the style, look and size,
`runWait` the rig doctor's run and route. Mock mode can not prove real timing; `?previews=slow` lands
Render previews one at a time.

A card moves at once (`state/move-card.ts`). A project's moves reach the core one at a time and the
Board takes the core's answer once the last has settled; after a failure each card goes back to the
last stage the core confirmed. `thumbnail_render_missing` answers with the files it queued, and the
Render previews wait ends when each of those has its `ThumbnailReady`, in whichever project is open.

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
under 400 code lines, no inline suppressions, no `warn` severities, and the core layer map (below).
Over-cap files can be frozen in `scripts/lint-meta/baselines/` while they are split; a frozen entry may
only shrink, and one that is no longer needed fails until it is removed.

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

### maquette-core layers

Every core module sits in a layer, and a layer only uses itself and the layers below it. The map is
`scripts/lint-meta/rust-layers.json`: a submodule such as `inbox::analyse` may sit in another layer
than its parent, and a path kept at its old place by a `pub use`, such as `pipeline::history`, sits
in the layer of the code it points to. `pnpm lint:meta` reads every `crate::` path in the core
against it.

| Layer     | May use    | Modules                                                                                                                                                                                                                                   |
| --------- | ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| base      | nothing    | `error`, `fs`, `json`, `toolchain` (the maquette-base crate), `app_state`, `base` (the run log and the Toolbox)                                                                                                                           |
| model     | base, glb  | `workspace`, `project`, `assets`, `avatar`, `layout`, `refs`, `timeline`, `settings` (the stored settings types), `sends`, `plans`, `recipes`, and the stored files of `prepare`, `built`, `materials` and `ship`                         |
| glb       | base       | `inbox::{analyse, fixture, images, math, vrm}` (the maquette-glb crate)                                                                                                                                                                   |
| preview   | base       | `site::preview`, `page` (the maquette-preview crate)                                                                                                                                                                                      |
| pipeline  | model, glb | `pipeline`, `prepare::preview`, `avatar::{normalize, reweight, springs}`, `built::{build, blender}`, `materials::{apply, split}`, `ship::{plan, report}`, `layout::view`, `thumbs`, `game`, `inspect`, `inbox`, `motion`, `parts`, `site` |
| providers | model, glb | `providers` (the maquette-providers crate)                                                                                                                                                                                                |
| lab       | model, glb | `concepts`, `refs::{files, suggested}`                                                                                                                                                                                                    |
| library   | pipeline   | `library`, `animations`, `packs`, `renders`                                                                                                                                                                                               |
| facade    | everything | `lib.rs`, `providers::tests` (the provider tests that drive a send into the inbox)                                                                                                                                                        |

A module whose file on disk is model data but whose work runs Blender or reads other layers is split
by submodule. `prepare.json`, `built.json`, `materials.json`, `avatar.json` and `ship.json` (their types, load,
save and checks) are model, so removing an asset can keep and clear its entries; the fix-up preview,
normalising an avatar's rest pose and freeing its arms, the built asset builds, applying material tweaks, the ship plan and
the size report are pipeline.
`layout::view` (the Inspector's read model, which measures prepared models) is pipeline, and
`refs::files` and `refs::suggested`, which read and mark the Lab's directions, are lab. Tests that
need a higher layer sit beside the code they test (`layout::view`, `prepare::preview`,
`refs::files`), and the Blender runs of the fix-ups and the built, material and ship tests are
pipeline.

A new module goes into the map, or lint-meta fails. The uses that point up today are frozen in
`scripts/lint-meta/baselines/rust-layers.json`, per file and module used: the count may only shrink,
and a new upward use fails. Moving shared code down a layer is how an entry goes away.

Where a layer needs work from a higher one, it asks through a port: a small trait in a lower layer
that the higher layer implements and the app hands in where it wires things up. Each port is one
call and does exactly what the code behind it did before. `providers` uses no higher layer; its tests
that drive a send into the inbox or run Blender sit in core's `providers::tests`, which is facade:

| Port (where the trait lives)     | Implemented by                        | Handed in by                                        |
| -------------------------------- | ------------------------------------- | --------------------------------------------------- |
| `base::ports::ScriptRunner`      | `PipelineRunner` (`pipeline::script`) | `add_model_file`, `inbox_add_model_file`            |
| `base::ports::PictureRender`     | `thumbs::PictureRenderer`             | `Arrivals::set_renderer` in `commands/downloads.rs` |
| `base::ports::TextureBake`       | `parts::bake_back::Bake`              | `SendPorts` in `ProviderJobs::new`                  |
| `sends::InboxPort`, `SendLinked` | `inbox::Inbox`, `InboxItem`           | `inbox::send_ports()` in `ProviderJobs::new`        |

`sends` holds what the inbox and the provider jobs share (`Provider`, `InboxLink`, `InboxAim`,
`inbox_dir`, `is_model` and the ports above); what a send asked for (`SendNotes`) and a file's hash
are in the base crate, and a game asset's folder budget lookup and a new game's settings are in
`settings::game`. The model layer uses no higher layer: a model's card numbers (`stats_of`,
`short_count`) and adding a model made in Blender as an asset's next version (`assets::versions`)
live in `assets`, and the timeline reads its models' parts itself.

Some layers are their own crates now. `maquette-base` holds `error`, `fs`, `json`, `toolchain`, the
run log and the Toolbox (`base::history` and `base::tools`, with `StepKind`), the ports
(`base::ports`), a file's hash (`pipeline::cache::hash_file`), what a send asked for
(`sends::SendNotes`), the clock (`project::now_rfc3339`) and a site's config (`settings::site`, a
stored settings type the preview crate reads too, so it sits in the base crate).
`maquette-glb` holds the glb layer's pure reading: `inbox::{analyse, fixture, images, math}`, the
part list of a model file (`parts::{PartInfo, read, read_bytes}`), its rig, clips and parts
(`motion::{Clip, Rig, BoneMap}` and the reader behind them) and its VRM readiness (`inbox::vrm`, see
VRM below). What core keeps of `inbox::checks`,
`parts` and `motion` uses model and pipeline types (`AssetKind`, `WorkspaceConfig`, `Fixup`, the
layout), so it is pipeline. `maquette-model` holds the model layer: `workspace`, `project`,
`assets`, `avatar`, `timeline`, `settings` and `sends` whole, and the stored part of `layout`, `refs`,
`prepare`, `built`, `materials` and `ship`. For those six and `avatar`, core keeps a module of the
same name that re-exports the crate's (`pub use maquette_model::layout::*`) and adds the work above
the model layer: `layout::view`, `refs::{files, suggested}`, `prepare::preview` and the fix-up script,
`avatar::normalize`, `avatar::reweight`, `avatar::springs` and `avatar::highlights` with their scripts,
`built`'s Blender build, `materials`' apply and split, and `ship`'s plan and size report. `maquette-preview` holds
`site::preview` and `page`. `maquette-providers` holds `providers`; core keeps a `providers` module
that re-exports the crate (`pub use maquette_providers::*`) and holds the provider tests that need the
inbox or a real Blender run. The crate's fake HTTP server (`providers::fake`) is behind its
`test-support` feature, which core's tests turn on. Core keeps every old path with a `pub use`, so `maquette_core::...` paths
do not change, and the map keeps those paths in their layer. A crate cannot reach up into core at
all: `scripts/lint-meta/crate-layers.json` lists the workspace crates each crate may depend on, and
`pnpm lint:meta` checks it against `cargo metadata --no-deps` (every dependency kind counts, and a
new crate has to be added to the list).

| Crate                | May depend on                                                                               |
| -------------------- | ------------------------------------------------------------------------------------------- |
| `maquette-base`      | no workspace crate                                                                          |
| `maquette-glb`       | `maquette-base`                                                                             |
| `maquette-model`     | `maquette-base`, `maquette-glb`                                                             |
| `maquette-preview`   | `maquette-base`                                                                             |
| `maquette-providers` | `maquette-base`, `maquette-glb`, `maquette-model`                                           |
| `maquette-core`      | `maquette-base`, `maquette-glb`, `maquette-model`, `maquette-preview`, `maquette-providers` |
| `maquette-desktop`   | `maquette-core`                                                                             |

### IPC type registration

Every crate, and every core module that owns IPC types, has a `register(types)` that adds its own
`specta::Type`s: `maquette_base::register`, `maquette_glb::register`, `maquette_model::register`,
`maquette_preview::register`, `maquette_providers::register` and so on. `maquette_core::types()` folds them in the order of its
`REGISTRIES` list, and the desktop crate's `bindings.rs` exports the result. A new IPC type goes
into the `register` of the module that defines it, so `lib.rs` changes only when a module is
added. The order does not change `bindings.ts`: the TypeScript exporter sorts the types by name.

| Registry                       | Types it adds                                                                                                                                                                                                                                                                                  |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `maquette_base::register`      | the error payload, the toolchain report, step kinds and the run log                                                                                                                                                                                                                            |
| `maquette_glb::register`       | the GLB report: its rig kind, textures, maps, islands, finger weights and VRM readiness                                                                                                                                                                                                        |
| `app_state`                    | the per machine app state                                                                                                                                                                                                                                                                      |
| `maquette_model::register`     | its modules' in turn: provider presets, the model catalog and game settings; how a model gets made; the workspace config; the project manifest; assets; the scene layout, object states and timeline; the Timeline view; the scene prep; built assets; the scene's materials; the avatar specs |
| `maquette_providers::register` | sends, handoffs, keys, batches, downloads and the ledger                                                                                                                                                                                                                                       |
| `layout`                       | the layout view                                                                                                                                                                                                                                                                                |
| `motion`, `inbox`              | the Motion view; the checks and the inbox                                                                                                                                                                                                                                                      |
| `pipeline`, `prepare`          | the pipeline definition, events and starters; the fix-up report                                                                                                                                                                                                                                |
| `built`, `thumbs`              | a build's result; the thumbnail state                                                                                                                                                                                                                                                          |
| `inspect`                      | the Inspect view                                                                                                                                                                                                                                                                               |
| `game`                         | a game repo's checks, scan, import and ship plan                                                                                                                                                                                                                                               |
| `concepts`                     | Lab concepts and jobs                                                                                                                                                                                                                                                                          |
| `renders`                      | render styles: plans, previews, loops and their records                                                                                                                                                                                                                                        |
| `library`, `packs`             | library entries, their uses and updates; export packs, their records and presets                                                                                                                                                                                                               |
| `maquette_preview::register`   | the Page view's settings and the dev server's command and status                                                                                                                                                                                                                               |

## MCP tools

The MCP server lives in `apps/desktop/src-tauri/src/mcp`: `server.rs` serves Streamable HTTP on
127.0.0.1 behind the bearer token, `tools.rs` holds the server instructions and joins one
`#[tool_router]` per file in `tools/` (`concepts.rs` starts and reads the Lab, `picks.rs` stars and
picks concepts onto the Board, `sending.rs` sends to 3D, and so on). A tool does what the matching
UI action does by calling the same `maquette-core` function or the shared command helper in
`commands/` (`pick_concepts` and Send picks to Refs both end in `concepts::picks::place_at_refs`),
so the checks are the same. After a change it tells the windows through `Host` (`project_changed`,
`concepts_changed`, `layout_changed` and the rest), which emits the same events the UI listens to.
The integration tests run a real client against a test `Host`. They are one test binary,
`apps/desktop/src-tauri/tests/mcp/main.rs`, with a module per area (`handshake.rs`, `picks.rs`,
`library.rs` and the rest) and the shared client helpers and plain test host in `support.rs`, so
the desktop crate links one test binary instead of one per file.

Tool groups: `tools.rs` tags each router with its group, and `tools/groups.rs` holds the fixed list
(`GROUPS`: id, title, a short line, and whether it is always on, which `start` and `project` are). A
test fails when a tool has no group in that list. Every session keeps the whole router; the setting
(`agents.rules.toolGroupsOff` in `maquette.json`, read through `AgentRules`; on open, an older file's `avatarRigTools: false` with no list folds into `["avatar-rig"]` and the key is never written) is read on each `tools/list`, which leaves the
off groups out, and on each `tools/call`, which refuses a tool of an off group with code `group-off`
before anything runs. Answers name no tool of an off group either: `groups::scrub_off_tools` drops an error's `tryTool` and the `next` steps of a success answer that point at one. `Agents::sync` compares the off groups with the last sync after every settings
save or workspace change and rings `Hub::tools_changed`; `tools/list_changed.rs` forwards that to
each connected session as `notifications/tools/list_changed` (a session's stream for protocol
versions before 2026-07-28, `subscriptions/listen` for later ones). The `mcp_tool_groups` command
gives Settings > Agents the groups with their tool counts, and the web mock mirrors them in
`ipc/mock/mock-tool-groups.ts`, held equal by a test.

The tool names have one source: the router in `tools.rs`. `cargo test -p maquette-desktop` writes
them to `apps/desktop/src-tauri/tool-names.txt` (`mcp/tool_names.rs`, like the IPC bindings), and
CI fails when the committed file is stale. The handshake test checks the served list against that
file, `scripts/check-skills.mjs` judges the skills by it, and the `tools-documented` rule in
`pnpm lint:meta` fails when the README or the docs site's MCP tools page does not name a tool in
it. A new tool goes into its `tools/` file and the instructions; then commit the regenerated
`tool-names.txt` and name the tool in the README and the docs page.

`tools/start.rs` (with `start/compact.rs`) holds `get_started` and `create_project`. `get_started` is short by default (counts, the toolchain as working, of and problems, the image backend, projects, blocking gates, next steps, the open project) and `detail: true` returns every toolchain row and every gate. `create_project` takes `target`, with `kind` as another name.

`tools/plans.rs`, `tools/plan_edits.rs` and `tools/plan_extras.rs` hold the eleven general planning canvas tools (`propose_removal` and `start_from_node` in the last). They go through
`maquette_core::plans` (see Planning canvases), and each write ends in `Host::plan_changed`, which
emits `PlanChanged` so an open canvas reloads and keeps the person's unsaved edits. Answers are short
on purpose: ids, counts and one line per node, with `full` for whole texts.

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
layout patch's `paths` and `pathTracks`, and the page's render style and hero text
tracks as `styleTrack` and `textTrack`.

`tools/inbox.rs` holds `list_inbox`, `get_inbox_checks`, `accept_inbox_model`, `discard_inbox_model` and
`render_thumbnail`; `tools/inbox_triage.rs` holds `accept_inbox_models` and `restore_inbox_model`. A discard
notes the asset the model was going to in `inbox/.discarded/discarded.json` (`inbox::restore`), so a restore
aims the model there again; `inbox::place` fills each model's `nextVersion` and `replacedBy` from its project.

`tools/library.rs` holds `list_library`, `add_to_library`, `add_many_to_library`, `use_from_library` and
`update_from_library`, which call the same `maquette_core::library` functions as the commands and
emit `libraryChanged` and the project events. `list_library` pages (a `limit` and an entry id `cursor`) and answers compact entries unless `full`. The send tools' `modelWarning` comes from the core `HasModel` send warning (`refs/checks.rs`), which the Send to 3D and batch panels show as a note beside the ref warnings. `tools/animations.rs` holds `list_animations`,
`import_animations` and `save_model_clips`; import paths must resolve inside the workspace.

`tools/rig.rs` holds `start_rig_doctor`, `run_rig_doctor`, `get_rig_doctor` and `list_rig_doctors`. They go through `maquette_core::rig`, which bundles the `maquette-character-rig` skill's scripts with `include_str!`, so there is one copy of them, and runs them headless on a copy in `cache/rig/<case>/`. The arrival read is `maquette_glb::analyse::rig_probe`, in Rust. No tool approves a joint move or lands a draft: `commands/rig.rs` does, and `mcp/hub/rig.rs` raises the approval card for a fix an agent started.
`tools/renders.rs` holds `render_styles`, `get_render`, `list_renders` and `decide_render`. They go
through the same `maquette_core::renders::Renders` service as the Inspector's Render tab and the batch
sheet: `renders/render_styles.py` runs headless through `PipelineRunner` on a lane of its own, `encode.rs`
joins the frames (WebP by its own RIFF writer, so a loop needs no ffmpeg; GIF and MP4 through ffmpeg with
the fixed palette, frame rate and dither), and a record per render lives in `exports/.renders/`.

`tools/packs.rs` holds `export_pack`, `get_pack`, `list_packs` and `decide_pack`. They go through
`commands/packs.rs` like the Library's pack sheet, with `from_anywhere` false, so an agent's pack
lands inside the workspace. `maquette_core::packs` plans a pack (the source closure that reads a
project's sends is passed in, since the library layer may not use providers), runs each model through
`export_pack.py` with the starter `fixups.py` beside it, renders the check with `render_thumb.py`,
compresses a web pack with the pipeline's compress step, and writes the README, manifest, contact
sheet and zip. A record per pack lives in `exports/.packs/`, and a folder Maquette wrote carries a
`.maquette-pack` marker, the only kind it replaces.

`tools/trash.rs` holds `remove_assets`, `list_removed_assets` and `restore_asset`, which go
through `commands/trash.rs` like the Board's Remove from project and Recently removed.

`tools/views.rs` holds `render_views` and `get_model_facts`, in the prepare group. They go through
`commands/views.rs` to `maquette_core::views` and `maquette_core::facts` (see Preview light, standard
views and model facts): the helper finds the asset version and its refs, and makes the copy with
the asset's fix-ups and real size through the same fix-up preview `preview_fixups` runs.
`render_views` waits on the views service up to `waitSeconds` and then answers `rendering`; the same
call again joins the run. Neither has a Tauri command: no view shows them yet.

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

## Codex usage

`maquette_core::concepts::codex_usage` reads how much Codex usage is left. `server.rs` starts
`codex app-server` in its own process group (a job object and no window on Windows), speaks its
newline delimited JSON (an `initialize` request, the `initialized` notification, then
`account/rateLimits/read`, no `jsonrpc` field), skips notifications and stops the server when the
read ends however it ends, within `READ_TIMEOUT` (10 s); its standard input is a pipe, so it also
ends if the app goes away. `parse.rs` reads the answer leniently into `CodexUsage` (the five hour
`primary` and weekly `secondary` windows, `limitReached`, `ordinaryUsageAllowed`). The protocol is
experimental, so a Codex older than `OLDEST` (the toolchain check's version), an error answer, a
timeout or a shape it cannot read is `CodexUsage::unknown` with a reason, never an error.
`limit_active`, `resets_at_ms` and `can_run_now` hold the rules, which `state/codex-usage.ts`
mirrors.

`concepts/jobs/usage_hold.rs` moves the jobs the usage limit holds: `retime_usage_limit` to a
read's reset time plus a minute (it beats the time `usage.rs` reads from Codex's message and the
backoff), `lift_usage_limit` back into the queue now. The desktop side is `commands/codex_usage.rs`:
`CodexUsageWatch` keeps the last read, `codex_usage_read` reads now (the Lab polls it every minute
while open), the Lab's event callback calls `read_soon` after a job is done, fails or meets the
limit (one read waits at a time, 10 s apart), and `lab_lift_usage_limit` is Limit lifted, retry now.
Every read goes out as `CodexUsageUpdate`. The MCP tools are `mcp/tools/codex_usage.rs`
(`get_codex_usage`, `retry_usage_limited_jobs`). The web side is `state/use-codex-usage-store.ts`,
`features/shell/CodexUsageItem` (drawn into the status bar through `codexControl`, like the site
server's item), `features/lab/UsageLimitBanner` and the Composer's round warning.

## Image backends

A Lab job is made by Codex CLI or by OpenAI's Image API. `concepts/backend.rs` holds `ImageBackend`
(`codex`, `openai-images`), the backend per kind of job (`ImageBackends`: concepts, object refs, other
views, clean background), `ImageSettings` (the backends, the API's quality and `monthly_cap_usd`,
kept in `maquette.json` under `lab.images`; Codex, low and $10 by default) and `ApiPlan`, which a
`Start` or `Lane` carries when its batch goes to the API. `commands/image_api.rs::plan` picks the
backend: the request's override (an agent's `backend`), else Settings for the kind.

`providers/openai_images.rs` is the client: a generation without attached images, an edit with
them (up to 16 as base64 data URLs, in the order Codex gets them: the brief's refs, then the style
board), one PNG per request, 429 handled by `http::Api` with `Retry-After` and `Timing::OPENAI`.
`price.rs` turns the reported tokens into dollars with the GPT Image 2.5 rates (`PRICES_CHECKED`,
`PRICES_URL`) and estimates a call that reported none. `test_key` lists the models and spends
nothing.

`concepts/jobs/persist.rs` keeps the Lab's queued, running, waiting and failed jobs in
`lab-jobs.json` in the app's config folder (`Concepts::keep_jobs_in`, called by `Lab::new`), with
what each run needs: the codex call (`codex_run.rs`) or the Image API request and plan. It is
written when a job joins, changes state or leaves, never on a log line. On start the kept jobs come
back failed with an Interrupted by a restart message (done when the image already landed), and
Retry or `retry_concept_jobs` reruns them; nothing resumes on its own.

`concepts/jobs/api.rs` runs the API jobs on their own in-process lane, as many at once as the Lab's
concurrency, and settles them through the same `finish` as Codex jobs, so the states, the file
layout, retries, cancel and `use_ref_candidate` do not change. Before a batch starts, the month's
spend (`ledger::month_to_date`, the local calendar month), what queued API jobs will likely cost and
the batch's estimate are checked against the cap, and a batch that would pass it is refused whole.
The cap check counts each image at `ApiPlan::per_image_usd`, what one has cost lately at that quality (`ledger::recent_average_usd`: the last 20 measured lines, once there are 3), else the price list's guess. A round the person starts has no approval: the cap is the guard. An agent's round is held: `image_api::plan` sets `ApiPlan::hold` for `via_agent`, `queue` puts its jobs in `ConceptJobState::Approval` instead of the lane, and `jobs/approval.rs` releases or denies them (`retry_with(id, true)` holds an agent's retry the same way). On the desktop side every MCP tool that queues Lab images ends in `mcp/tools/image_round.rs::ask_for_round`, which adds an `ApprovalKind::ImageRound` to the hub (the jobs and `estimate_usd`), answers `waiting for approval`, and waits on a thread for the decision, withdrawing the approval once no job is left waiting (cancelled in the Lab). The web side shows it in `features/agents/ApprovalCard` and the queue rows. A 400 from OpenAI's safety system (`openai_images::refusal`; `http` keeps OpenAI's `error.code` in the message) fails the job with `ConceptJob::refused` and a message to reword the prompt, and writes nothing to the ledger. `get_credits` adds `openaiImages`, the month's dollars against the cap. Every made image appends an `ImageSpend` line
(v3, `event: "image"`, dollars, tokens, estimated or not, via agent or not) to `credits.jsonl`;
older readers skip it. A job cancelled while its request is out still records the spend when the
answer comes, and its image is dropped. A kept 429 becomes a rate limited job like Codex's.

The key is in the keychain under its own account (`secrets::OPENAI_ACCOUNT`), never in a file.
The desktop commands are `image_api_status`, `image_api_key_save`, `image_api_key_remove` and
`image_api_key_test`; the MCP tools are `mcp/tools/image_api.rs` (`get_image_backend`,
`set_image_backend`, which emits `ImageSettingsChanged` and refuses `monthlyCapUsd` with `CAP_REFUSED`: the cap is set only in Settings), and `start_concepts`, `make_object_refs`,
`make_other_views` and `clean_ref_background` take `backend`. The web side is
`state/image-api.ts`, `state/use-image-api-store.ts`, `features/settings/ImagesSection` (the key row
is the shared `KeyRowView`) and the Composer's cost line. Debug builds point the client at
`MAQUETTE_OPENAI_URL`; tests use a fake server and never call OpenAI.

## Game projects

`ProjectManifest.target` is `website` or `game`; a game keeps `GameSettings` in `game` (boxed, so
the events that carry a manifest stay small). `maquette-core/src/game/` holds the rules: `detect.rs`
reads a game repo, `scan.rs` and `import.rs` turn its models into assets (version 1 is the repo
file, recorded in `game.models`), `budget.rs` checks models against their folder's budget,
`index.rs` writes `web/game-index.json` before every run (each model's target path, whether it is
still the untouched import, its `textureCap` from its folder budget with the global cap as the
fallback, the export settings), `ship.rs` plans what Ship would change against
`web/ship-record.json` (a replaced file whose game copy is still the imported one is `imported`;
`detect::uncommitted` counts `git --no-optional-locks status --porcelain`, read only), and `settings.rs` saves a settings change and re-ensures the pipeline. `GameSettings.folders` moves
one asset to a folder of its own (`file_of` keeps the file name); changing it writes the old path
into the ship record's `remove` list (`ship::record_moves`), so the plan shows it as removed and
`ship_game.mjs` deletes it only when Maquette wrote it. `detect::check_repo` checks a path an agent
names as a game repo (a checkout or a worktree's `.git` file), for `set_game_settings` and for the
one plan or run that `get_ship_plan` and `run_pipeline` take a `repo` for. `GameExport.drop_maps`
has `export_game.py` unhook the normal and metal-roughness textures; each rig's export keeps only
its own clips. Budget fix-ups never give a
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

`maquette_core::timeline` builds the view (the stops, every model with its parts read from its
newest model file, the states and the tracks) and saves it: `with_timeline` replaces only the stop and
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
  the hips by height (each clip imported and keyed at its own frame rate, the model exported at the
  highest) and measures each clip (travel, how far the hips go, each foot's track);
  `feet.rs` finds planted frames (low and slow) and the slide in those tracks, and for a clip with
  Pin the feet a second run holds the feet over the planted frames with a two bone leg IK;
  `review.rs` grades them. `bake` adds the draft as the next version through the build checks.
  `uses.rs` keeps the project's `animations.json` (saved map, baked clips with revision and
  settings, last review), which gives the library's uses and update offers.
- **Edges.** The Tauri commands are in `commands/animations.rs` and emit `libraryChanged` (and
  `projectChanged` after a bake); the MCP tools are `mcp/tools/animations.rs` and
  `mcp/tools/retarget.rs`; the web state is `state/animations.ts`, `state/retarget.ts` and their
  stores, the views `features/animations` and `features/retarget`.

### Recipes

A recipe is a base clip from the library with changes laid over it, baked later into one clip of a
model's next version. The file is model data (`maquette_model::recipes`, re-exported as
`maquette_core::recipes`): one JSON file per recipe at `<project>/recipes/<id>.recipe.json`, the id
made from the name and fixed for its life (Rename only changes `name`, Remove moves the file into
the project's trash). It holds `version`, `name`, `revision` (1, moved only by a later save back
to a library of recipes), `created`, `edited`, `base` (the clip's name, frames, length, hash and
`mixamo`), `blanks` (targets filled per project: a `prop` asset id or a `point`) and `layers`
(`reach`, `nudge`, `look-at` or `wave`, each on a VRM humanoid bone name with its settings). It
never holds keys, frames, a mesh, a skeleton or a render, so it can be shared. Reading normalises
it (trimmed text, each change keeping only the settings its kind reads) and checks it whole; a file
over 256 KB is refused unread, like a canvas over its cap, and a write that would make one is
refused (`recipe-too-big`).
`animations::recipes` (library layer) makes a recipe from a clip and matches a recipe's base
against the library by name and frame count: `found`, `hash-differs` (a warning, not a refusal)
or `missing`. A bake counts as Mixamo's when the recipe says so or its matched clip came from a
Mixamo rig. The Motion view's Recipes tab lists them, and New recipe opens the base clip picker
(`features/animations/RecipeClipPicker`, mounted by the shell with the other animation sheets).
Agents use `list_recipes`, `get_recipe`, `create_recipe` and `update_recipe` in the animation
group; each write ends in `Host::recipe_changed`, which emits `RecipeChanged`.

Version 2 adds `joined` clips after the base (each with a `blend` window and its line up), a `trim`
on any clip, `mirror`, `time` edits (retime, freeze, hold over) on a clip's own frames, and the pin,
pose key and grip changes. Each is written only when used: a save writes version 1 while a recipe
uses none of it, so an older Maquette still opens it, and version 2 once it does; a version past 2
is refused. Pins, pose keys and grips all bake. Keys and time edits sit
on a clip by its index and its own frame. A change's `start` and `end` are shares of all the clips'
own frames laid end to end (on one clip, of its length), so a trim, retime or freeze leaves it on
the motion it covered; `recipes/clip_moves.rs` moves every share, key and time edit with its clip
when clips are joined, moved or taken out, and refuses (`clip-in-use`) taking out a clip one sits
on or parting the clips a change runs across. `recipes/timeline.rs` is the one source of frame
math: which frame of which clip each frame of the bake shows (trims and time edits on each clip,
then the clips joined across their windows), the seams and what each lines up, and where each share
stands. A recipe whose whole timeline runs over `MAX_TIMELINE_FRAMES` is refused by the checks,
counted without building it. The web app keeps step for step copies in `state/recipe-timeline.ts`
and `state/recipe-shares.ts`; `recipes/fixtures/timeline.json` and `fixtures/clip_moves.json` hold
the cases both must agree on.

Pose keys (`recipes/pose_keys.rs`) stand at the share of their clip's own frame among all the
clips' frames, so they ride with trims, retimes, holds and joins like a change's span (on the rest
pose a key's frame is the recipe's own). Each bone blends only between the keys that turn it, as the
later key's ease says (smooth, in, out, linear, step), holding its first key before and its last
after. A turn is degrees about the model's own axes, X then Y then Z, under the bone's parent: add
lays it on the clip, replace turns from the rest pose, both by the change's weight with no ramp.
`recipes/key_edits.rs` adds, moves and removes keys for `set_recipe_keys`;
`recipes/joint_limits.rs` holds the fixed humanoid table (left side and middle, the right
mirrored) that judges keyed turns, warn only. The bake stages `recipe_keys.py` beside
`bake_recipe.py` with each pose key change's keys as shares, and its review adds a joint limits
line. The web app's copies are `state/recipe-keys.ts` and `state/joint-limits.ts`, checked against
`recipes/fixtures/pose_keys.json`, and the rough preview lays keys with the bake's math
(`features/viewer/recipe-keys-pose.ts`) under a turn gizmo (`turn-gizmo.ts`).

`animations::recipes::bake` bakes a recipe onto a rigged model in one headless run of
`bake_recipe.py`, staged under the workspace cache and added as the model's next version only after
a review. A recipe that plays its one clip whole goes onto the model through the retarget, as Use
on model does. Any other carries its timeline in the job: each clip is retargeted whole as an
action of its own, and `recipe_motion.py` builds the new clip frame by frame from them (each clip's
pose at its frame, holds turned toward their frame, a joined clip lined up with the clip before at
its seam, blend windows, then mirror) before the changes are laid over it at the frames their
shares stand on.

Pins are held last, after every other change on each frame. `recipes/pins.rs` puts each on the
timeline's frames (from the first key its `start` reaches to the last within its `end`), eases it
in and out (`easeIn` and `easeOut`, 3 frames unless set, 0 holding at once) and says what of a foot
stays flat (`flat`: toe and heel unless set, heel keeps only its heading, off none; a hand keeps the
turn it had). A pin stays where the bone is at its first frame, else on its blank's target. The job
carries the spans, and `recipe_pins.py` holds each with an IK constraint to an empty on the spot,
chained to the upper limb, and a copy rotation for the turn, then measures how far the bone drifts
from the spot over the frames it holds whole and how far it would move without the pin. The review
reads that as a Pin drift line per pin (`pin-<layer>`, 1 cm). The web app's step for step copy is
`state/recipe-pins.ts`, held to `recipes/fixtures/pins.json`.

A write that leaves the blanks out keeps them as stored, and a prop blank sent open whose stored prop
is no longer an asset keeps that prop: only reads open it (`RecipeRead::gone` lists them), so
restoring the asset fills it again, unless the write names the blank in `clear`. A write may carry
`expected`, the `version` it read (`RecipeRead::version`, the first 16 hex digits of the SHA-256 of
the file's bytes): the core compares it under the write lock and refuses a moved file with the code
`moved`, writing nothing, so a hand edit or a sync that keeps `edited` is caught too. `update_recipe`
refuses lists sent without it (`needs-version`); a name only update may leave it out.
A change sent without an id gets the lowest free `l-N`; the editor gives the entries a person adds a
random five character tail (`l-k3x9q`, `prop-...`) that has a letter, never reused, so the two never
meet.

The editor's state is `state/use-recipe-editor-store.ts`: the file as last read (the base) and the
draft over it, so leaving the tab keeps the edits. Leaving the recipe with unsaved edits asks first,
as a canvas does. `state/recipe-check.ts` runs the core's checks on the draft as it is typed;
`state/recipe-merge.ts` merges an outside write with unsaved edits by change and blank id and asks
only on a clash (an id both sides added with different entries is two additions: the person's is
renamed). Every side's changes are moved onto the clips the merge lands on before they are
compared, and one that cannot sit on them is a clash, and so are the clips: Keep mine keeps the
person's clips with it. Save reads the file first, sends only the lists that changed with
`expected`, and on `moved` reads and merges again for the person to save once more. The rough
preview is
`features/viewer/RecipePreview` (lazy `RecipeStage`): the base clip through the rough retarget, then
`features/viewer/recipe-pose.ts` lays the changes over (two-bone IK for reach, an aim for look at,
an eased turn or move for nudge, a sine for wave), and `check-reaches.ts` measures each reach's end.
`features/viewer/recipe-pins.ts` then holds the pins roughly with the same two-bone IK, from spots
taken at each pin's first frame, and checks how far the hips move against the limb's reach for the
pin's settings. Pins have a Pin lane of their own in the editor, pose keys a lane per keyed bone,
grips a Grip lane named from the library. The editor adds a grip from `features/motion/GripPicker`
(the library through `state/use-grip-library-store.ts`, drawn by `features/common/GripPicture` as a
schematic hand) and edits it in `RecipeGripForm`, which offers a newer revision (`gripNote` in
`state/recipe-library.ts`) and never swaps it on its own. `state/use-grip-save-store.ts` backs the
Save a grip sheet (`features/animations/SaveGripSheet`): it reads the pose keys at one frame from the
editor's draft or the file, plans after each choice and saves against the version it planned, and
an agent's `propose_grip_save` opens the same sheet from its approval card. The Animations view's
Grips part is `features/animations/LibraryGrips`.
A joined recipe plays the timeline: each frame poses the clip frames it names, blends across a
window and holds a frozen pose, the same frames the bake makes.
Facing comes from the rig (`rigForward`: up crossed with left to right upper arm, else upper leg,
else +Z) and drives the wave axis, the look bones, where markers sit and so the gap check. The rough
retarget itself works in world space, so a model facing +X under a +Z clip has its limbs turned to
the clip's world directions. Each setup starts from the bones as they came and stops the old mixer.

## VRM

`maquette_glb::vrm` (kept at `inbox::vrm` in core) reads how far a rigged model is from a VRM 1.0
avatar, without Blender. It is pure: the `gltf` crate and the matrix math the analysis already
uses.

- **Bones.** `bones/table.rs` holds VRM's 55 humanoid bones (15 required) with their parents and the
  names four namings give them: VRM's own and VRoid's `J_Bip_` names, Mixamo, Unreal, and Tripo's
  own rig (unverified, as in `animations::rig`), and `bones.rs` maps the joints onto them. Joints are compared by bare name in lower case;
  the naming with the most matches is tried first, a joint maps to one bone, and an override (a
  bone to any node name, or to none) wins. `bones/descriptive.rs` then reads descriptive names
  (`Left elbow`, `Upper arm L`, `ThumbProximal_L`) into words, a side and a joint for the bones
  still empty, taking a joint only when it alone reads as that bone. `naming_report` gives the
  `NamingReport`: the winning naming, the required bones it matched, the joints that look
  humanoid, the ambiguities (two joints for one bone, one joint for two) and whether the map is
  confident; an unclear map is a warning that asks a person to assign the bones, never a guess.
  The parent check walks both trees up to the nearest mapped bone, so other nodes may sit
  between.
- **Skin.** `skin.rs` reads every skinned mesh and primitive, every `JOINTS_n`/`WEIGHTS_n` set in
  any component type: joint indices past the skin, unweighted vertices and weights off 1 by more
  than `WEIGHT_SUM_SLACK` block, as do a joint listed twice in a skin and a loop in the node tree;
  more than `MAX_INFLUENCES` influences, limb bones without length and sides that differ warn.
  A sheared node matrix counts as an uneven scale in `pose.rs`.
- **Rest pose.** `pose.rs` measures the nodes' rest transforms: the facing from the left minus
  right upper legs and arms (left crossed with up; the toes must agree or the sides look swapped),
  each arm's drop below horizontal, the soles (a skinned mesh's bounds through its first joint's
  world matrix and inverse bind matrix), the hips' distance from the Y axis (also split into side and
  forward in the character's own frame), mirrored bone gaps in that frame with the bone of the
  widest, and every node's scale (negative, uneven, evenly scaled, or float noise within
  `SCALE_NOISE`, 1e-4: exports leave up to about 2e-5).
- **Findings.** `findings.rs` turns both into blockers, warnings and notes in plain words, sorted
  by level and never dropped. `VrmReadiness` carries the bone list, the pose and the findings; it
  rides on `GlbReport::vrm` (the inbox caches it under a report version) and `Rig::vrm` (Motion's
  rig check). Core's `inbox::checks::vrm` makes the one Inbox row from it, a pass or a warning.
- **Fixture.** `fixture::humanoid(&HumanoidSpec)` builds a box humanoid on the 65 joint skeleton
  under any of the five namings (the fifth descriptive), optionally in an A-pose, turned, lifted,
  scaled, or with float noise in its armature's scale; its skin in float, 8 or 16 bit weights,
  split over two meshes, or with one `SkinFlaw`.
- **Writer.** `vrm::write(glb, &WriteSpec, base)` returns VRM 1.0 bytes and the pose findings the
  file still carries. It refuses a rig VRM cannot hold (no skin, a required bone missing, a gap,
  a wrong parent, swapped sides, a negative, uneven or sheared scale, a loop in the node tree, or
  a skin blocker) and a meta `VrmMeta::validate` refuses
  (no name, no author, another licence URL); facing, arms and ground do not stop it, since
  normalising the rest pose is a step before. `container.rs` splits and joins the GLB chunks and
  only adds to them: `VRMC_vrm` (meta, `humanoid.humanBones` by node index, `firstPerson` auto on
  every mesh node, a default `lookAt`, bone when both eyes are mapped and expression otherwise),
  `extensionsUsed`, and a thumbnail appended to the BIN chunk as one more buffer view and image.
  The animations go (their keyframes stay in the BIN chunk, unused), node scales within
  `SCALE_NOISE` of 1 snap to 1, and `MaterialMode::MtoonSoft` (`mtoon.rs`) puts
  `VRMC_materials_mtoon` on every material, shaded from the base color and texture with no
  outline, keeping the PBR values as the fallback; `Pbr` leaves the materials as they are.
  `base_color.rs` reads the base color images out of written bytes and swaps in others, adding
  the new image and texture and pointing the base color and MToon shade at them, so the original
  stays in the file. The
  golden test compares the written fixture's JSON with `vrm/golden/humanoid_vrm.json`
  (`MAQUETTE_BLESS=1` writes it again).
- **Avatar spec.** `maquette_model::avatar` keeps each character asset's `AvatarSpec` (the
  `VrmMeta`, bone map overrides, the material mode and what MToon does with baked highlights) in
  `avatar.json` next to `project.json`,
  keyed by asset id like `materials.json`, with the same versioned load and `update` that reads the
  file on disk first. A stored spec may be a draft: `VrmMeta::check_draft` checks the licence URL,
  blank authors and text length, and the name and authors are only required by `VrmMeta::validate`
  when the VRM is written. `WorkspaceConfig::avatar` (`AvatarDefaults`: author, contact, copyright,
  every licence field at the VRM default, the material mode) fills a new spec through
  `AvatarDefaults::spec`. Removing an asset keeps its spec in the trash record, and restoring it puts
  the spec back.
- **Normalising the rest pose.** Core's `avatar::normalize` (pipeline) runs `avatar_normalize.py`
  headless through the `ScriptRunner` port (`run_step`, so the run's one step is named for what
  runs and a failure carries the script's own `[avatar] Error:` line) on one asset version, only
  read: it turns the model to
  face +Z, centres the hips, puts the soles on Y = 0, lifts each arm at the shoulder until upper arm
  to hand is level (`Elbows::Straighten` levels the lower arm too), straightens each finger chain
  along its hand (`Fingers::Straighten`: the four fingers in the palm plane with half their spread,
  the thumb 45 degrees towards the front, trying a whole, three quarter, half and quarter turn per
  hand and keeping the first whose finger-weighted edges stretch at most 1.35 at the 95th
  percentile; hands whose finger bones sit bunched at the wrist are skipped), applies the pose as the rest
  (armature modifier applied, `pose.armature_apply`, modifier added back, at most 4 influences,
  weights summing to 1, weights otherwise untouched) and exports a plain GLB, with a report of the
  measures before and after and two pose sheets (rest, arms down, arms raised, a forward step; front,
  side, back and the shoulders, plus a row with each hand from the thumb side and from above). The bone roles come from the readiness's bone map, so any naming
  works, and `plan` refuses a rig with a blocker normalising cannot fix. The result is derived,
  never a version: `<cache>/avatar/<project>/<asset>/v<version>/`, keyed by the source's content,
  the script, the bone map and the options, recorded with the step cache so a fresh result skips
  Blender. The readiness is read again on the output, which must have both arms within
  `ARM_SLACK_DEG` of level and face +Z. A result starts `Pending`; `approve` records the person's
  yes against that key and the GLB's hash, and `approved_glb` and `write_vrm` refuse anything else.
  The skill `maquette-avatar-vrm` carries a copy of the script, which a test keeps the same.
- **Freeing the arms.** Core's `avatar::reweight` (pipeline) runs `avatar_reweight.py` the same
  way, before the normalise, when the readiness has the `arm-weights` finding (cloth or hair the
  arm chain holds that sits more than twice the arm's own radius from it, the radius read from the
  mesh around each arm segment in `arm_weights.rs`). The script cuts the mesh in slices along each
  arm, keeps the loops that hug the bone and follow on from slice to slice (sleeves, gloves, the
  hand, a cuff or a separate glove whole; the hand's own surface, reached over mesh edges from the
  arm at the wrist through what lies past it near the bones, always stays, with a small piece of
  its own lying on it or weighted mostly to it, while hair that only hangs beside the hand does
  not), and moves every other vertex's arm weight to spine, chest and upperChest (and the
  shoulder above the armpit) by inverse distance, with a geodesic band at the boundary so cloth
  sewn to a sleeve stretches instead of tearing. Hair the rig also hangs from the head keeps
  hanging from it: where most of a vertex's weight is on the head, the moved weight goes to the
  head instead, fading to the trunk over the surface below (`head_share`), and only on vertices
  that already carry some head weight, so a ponytail on the shoulder still turns with the head
  and the springs finder roots it there. Only `JOINTS_0` and `WEIGHTS_0` change, at most
  4 influences, except that the bridges (faces fused between an arm and the body far from the
  shoulder, which stretch whatever the weights) leave their primitives' index buffers when the
  spec's `bridges` is `Hide` (the default): the kept indices move to the front of the accessor and
  its count shrinks, so no vertex, UV or weight moves, and the JSON chunk is written again. The
  choice is part of the key and recorded on `Reweighted`, whose `approved` check reads it back;
  `finish` refuses an output with unweighted vertices or
  more influences. The result lives in `v<version>/reweight/`, keyed and cached like the
  normalise, with its own before and after sheets (rest, arms level, arms raised). Both share
  `avatar/gate.rs`: a result starts `Pending`, and a yes holds only for the output bytes it was
  given for. `normalize::plan` lifts the approved reweight when there is one (its key then reads
  that GLB), and the `.vrm` report records it. `AvatarRequest.reweight` (`Auto` or `Off`) decides
  whether a build frees the arms first; a waiting reweight answers `ReweightWaiting`, and a yes or
  a no both go on to the normalise. The desktop's `avatar_decide_reweight` and the hub's
  `ApprovalKind::Reweight` carry the person's answer; the web shows it in
  `features/avatar/ReweightStep`. The skill carries a copy of this script too.
- **Spring bones.** Core's `avatar::springs` (pipeline) runs `avatar_springs.py` on the model a
  VRM is written from (the approved normalised rest pose, or the version's file when its rest pose
  is VRM's already; `export/springs.rs` picks it), in two modes. `find` builds a body model from
  the humanoid bones (a capsule per segment, its radius read per direction from the mesh around it,
  low percentile, mirrored and capped so hair does not inflate it), takes the vertices well off
  that body and away from the head's own volume, joins them into parts by a voxel grid, gives each
  part a root (the humanoid bone its welded neighbours at the top are weighted to, else its own
  top's weights), drops parts that do not hang, and draws a centre line down each by a walk from
  the attachment. Each candidate has a kind (hair from the head or neck, cloth when long, an
  accessory otherwise), a line, a length, a vertex count, a confidence (how much it hangs, how much
  lies below the attachment, how slender and how big it is) and a colour on `candidates.png`.
  Small pieces close to the body come after them (`find_small`): vertices standing out of the
  surface around them (`local_rise`: the distance from the bone less the median of that stretch
  and direction, or the body radius where the mesh is too coarse), joined by a finer voxel grid,
  taller than wide and hanging down, left out on the forearms, hands, shins and feet. Each is a
  `charm-N` accessory flagged `small` with 2 to 4 joints by default, a close look on the sheet,
  a fade back over a quarter of its length where it touches the rest, and a near look on the sway
  test. It is cached per model under `v<version>/springs/find/` and never waits for a yes. Nothing is
  confirmed on its own: `AvatarSpec.springs` (`maquette_model::avatar::SpringSpec`: id, kind, root
  bone, line, joints, preset and swing overrides) holds what a person or an agent confirmed, and
  `AvatarSpec.no_springs` that they confirmed none (`springs_choice`: unpicked, none or picked).
  `chain` reruns the finder, matches each confirmed spring to a part by root and line, adds N
  joints along its line (`spring_<id>_<k>`, parented to the root's joint, by length unless the spec
  says), weights the part to them with a smooth blend between joints, a ramp from the root at the
  attachment and a fade where the part touches the rest, keeps at most 4 influences, and derives
  colliders (head sphere, capsules for neck, chest, hips, upper arms and legs, sized from the body
  model) in groups. Only new nodes and `JOINTS_0`/`WEIGHTS_0` change; `finish` refuses unweighted
  vertices, more influences or a different humanoid bone map. Its key covers the parts and joint
  counts only, so tuning a preset or a swing value needs no new run. It is its own approval step
  on `gate.rs` under `v<version>/springs/chain/`, with a sheet of the parts and their joints and a
  sway test (rest, swung left, right and back). A build with `SpringsMode::Auto` writes no springs
  when none are confirmed; with nothing picked it runs the finder and, when the last export carried
  springs, answers `SpringsUnpicked` instead of writing without them. With picks it runs the finder
  too and holds each pick to its candidates (`export/rematch.rs`): a pick whose part kept its root
  and line stays, one the finder names differently now is carried over to the candidate on its
  line hanging from about the same place (saved to `avatar.json`, keeping kind, joints and preset,
  listed as `carried`), and with no clear match the build answers `SpringsStale` and writes
  nothing. It chains the picks, answers `SpringsWaiting` while the chains are pending,
  and after a yes writes the VRM from the chained model and adds `VRMC_springBone` 1.0
  (`maquette_glb::vrm::add_springs`: springs with per joint `hitRadius`, `stiffness`,
  `gravityPower`, `gravityDir` and `dragForce` from the kind's preset with the spec's overrides,
  colliders and collider groups); after a no it is written without. The round trip counts and
  checks the springs, `AvatarExport.springs` and the report's `springs` record them. The desktop's
  `avatar_find_springs` and `avatar_decide_springs` and the hub's `ApprovalKind::Springs` carry
  the steps; the web shows them in `features/avatar/SpringsPanel`, and Preview's Wiggle shakes the
  hips and head so three-vrm swings the springs. The skill carries a copy of this script too.
- **Baked highlights.** Core's `avatar::highlights` (pipeline) runs on every MToon build, after
  the VRM is written and before it is saved, unless the spec's `highlights` is `keep`. It reads
  the base color images out of the written bytes (`maquette_glb::vrm::base_color_images`), writes
  them to `<asset avatar folder>/highlights/` and runs `avatar_highlights.py` once over them,
  cached by their content, the script and `STRENGTH`. The script measures on a copy at most 2048
  wide: a pixel is a highlight when it stands out above an opening by reconstruction of the luma
  (a square 1/32 of the width, so wide light areas and anything joined to them are kept whole),
  the mean colour of the not standing out pixels around it is dark, and it is grey, near the line
  from that colour toward white and not warmer than it (skin leaves that line). Each test is a
  smooth ramp; the product moves the pixel `STRENGTH` (0.75) of the way to the colour around it,
  feathered a pixel. An image with under 0.2% highlights gets no copy. The copies go back in with
  `swap_base_color`, which adds an image and texture and points the base color and the MToon
  shade at them, so the original stays in the file unused. A failed run never stops the build:
  the texture is kept and `HighlightsReport.note` says why. The report, `AvatarExport.highlights`
  and the Export panel's Last export show the mode and each image's coverage.

## Downloads matching

`providers/arrivals.rs` is the Downloads watcher. `matching.rs` places a file by its export name
and suggests a send for a close name or the only send waiting, however late it came. Every other
download is ranked against all waiting By hand sends: `arrivals/render.rs` queues a headless front
render of a GLB copy (`render_thumb.py --view front --plain`, transparent ground) on its own runner,
one at a time and cached by content hash under `<workspace cache>/download-pictures`, and
`arrivals/picture.rs` scores it against each send's front ref (outline IoU after scaling both to
one square, colour histogram overlap, box ratio; the name only breaks ties) into
`Arrival.suggestions`. A clear best (at least `LOOKS_LIKE` and `LEAD` ahead) turns the download into
Needs a look; nothing is attached by itself. `refs/picture.rs` is the object reader both this and
the ref checks (`refs/checks/agree.rs`) use. The render is a `PictureRender` port: the desktop sets
`thumbs::PictureRenderer` in `commands/downloads.rs`; the web shows it in
`features/providers/ArrivalPicker`.

`providers/own_file.rs` adds a model file someone fixed by hand as an asset's next version: it
checks the path (outside the inbox and every `models` folder), turns a `.gltf`, `.fbx` or `.obj`
into a GLB headless through the `ScriptRunner` port it is handed, and hands it to
`ProviderJobs::attach`, the same path an attached download takes. `add_model_file` and
`inbox_add_model_file` call it.

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

## Removing a project

`project/trash.rs` in `maquette-model` moves a whole project out of the workspace without deleting
anything, and the app is the only caller: no MCP tool removes or restores a project.

- **Refusals.** It is refused with the reason while work runs in the project: a pipeline run or
  queued run, a built asset building, concept images queued or generating, or a send that is not
  over (waiting for approval, queued, being made, waiting for a By hand model). The page preview dev
  server is stopped and the plan file watchers are dropped. The dialog greys out Remove project
  with the reason.
- **Moves.** Every folder moves with `maquette_base::fs::move_tree`: a rename, tried again on
  Windows while a file is held open. It copies and removes only across volumes, and when that
  removal stops partway it keeps both folders and the error names them. A failed rename moves
  nothing.
- **Files.** The project folder moves to `<workspace>/trash/_projects/<id>-<date>-<time>/` as
  `project/` (its own asset and canvas trash stays inside, under `trash/`), with a `removed.json`
  beside it. The site or game repo is never touched, and providers.json keeps the sends' records.
- **Restore.** It moves the folder back and is refused when a project with the same id exists
  again. The check and the move share a lock with `project::create`, so a new project cannot claim
  the folder in between. A repo path that comes back is marked as set by an agent, so its dev
  command waits for a person again, and so does an agent's Ship into it. There is no permanent
  delete, as the asset trash has none.
- **Agents.** A removed project is left out of `list_projects`, and a tool given its id fails with
  not-found, saying it was removed to the workspace trash and only a person can restore it.
- **Edges.** The Tauri commands are in `commands/project_trash.rs`. The web state is
  `state/use-project-trash-store.ts`, with `RemoveProjectDialog` and `RemovedProjectsDialog` in
  `features/project`. Removing the open project opens the next in the switcher.

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

## Preview light, standard views and model facts

What an agent needs to judge a model against its ref and to replace one model with another,
without a script of its own.

- **The rig.** `thumbs/preview_rig.py` is the one page light: Build scene's sun (3.0, tilted 0.9
  radians, mirrored to the front left) and grey world (0.18), rendered in Cycles to a float EXR and
  shown through three.js' ACES Filmic curve at exposure 1.15, written as sRGB. The curve is applied
  in numpy, so no Blender view transform takes part. `scene_color` undoes it for a backdrop that
  must show as its hex, and `ground` makes a floor that only catches a shadow. A test in
  `thumbs/tests.rs` holds the numbers to `starter/build_scene.py` and to the site scene's
  `Workbench.tsx` (listed in `scripts/gate/plan.mjs`, so a change there runs the whole gate).
- **Thumbnails.** `thumbs/look.rs` picks the look per asset: `ThumbLook::Page` for every version
  of an asset `built.json` lists (a provider version made before it was built too), the studio for
  every other asset. `render_thumb.py --look page` keeps the backdrop, the
  floor and the camera and takes the light and the tone curve from the rig. A page render is cached
  as `<hash>-page-<rig key>.png`, the studio's as `<hash>.png` as before, so a changed rig renders
  the built assets' pictures again and nothing else. `commands/thumbs.rs::ask_version` asks for the
  look of the asset; the Inbox and `render_thumbnail` stay studio.
- **The swatch sheet.** `thumbs/swatch_sheet.py` renders a palette as tiles in the rig, each with a
  strip of the colour as written, and reports what each tile shows. The docs' reference picture
  (`apps/site/public/docs-media/page-light-swatches.png`) is made by the `swatch_sheet` example,
  and a Blender test holds the pale tiles to what the reference page shows for the same baked tiles.
- **Standard views.** `maquette_core::views` (library layer): `cameras.rs` is the set (three
  quarter at 38 and 22 degrees, front, top and two close ups whose targets are a half, a named
  object or a box in shares of the bounds), `spec.rs` the JSON `views/render_views.py` is told and
  the key of a set, `job.rs` the `Views` service on the `views` runner lane. A set lands in
  `<project>/exports/renders/<asset>/views/v<n>/` (`v<n>-vs-v<m>/` side by side, where both models
  share cameras fitted to their joint bounds; `-<8 hex>` of the cameras when they are not the
  standard set, so another angle never shares a folder or a run with it) with `sheet.png` and
  `views.json`; the pictures are rendered in a scratch folder under the workspace cache and copied
  over together, the old `views.json` removed first. `views.json` keeps the key (scripts, size,
  views, the models' and refs' content), so the same request answers from the folder, one for the
  same set on its way joins it, one for another key in that folder is refused until it lands, a
  failed run says why once and `cancel` stops one.
- **Facts.** `maquette_core::facts` (pipeline layer) measures nothing anew: counts and the file's
  box from the Inbox analysis, the real size and the box at real size from `layout::prepared` (the
  Layout editor's own), the entry and the box in the scene from `layout.json`, and the facing from
  `facts/facing.rs`. A rigged model's front is the rest pose's (`how: rig`, `sure: measured`); a
  prop's is glTF's +Z and says so (`how: convention`, `sure: assumed`). `placed` turns it by the
  layout entry with the matrix the layout places bounds with.
- **Axes.** glTF and three.js share one frame: Y up, the front at +Z. Blender imports that as Z up
  with the front at -Y, X unchanged. The `signpost` fixture (`maquette-glb`) has a green marker on
  +Z, a red one on +X and a blue one on top, and the Blender test of the views checks that the front
  picture shows green in the middle with red to its right and the top picture blue with green below.

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

Ship approval: a website run a person starts never waits before Ship. A run an agent starts gets
its gate from `commands/pipeline/run.rs::agent_run`, which `run_pipeline` calls. Its `ShipGate`
(`mcp/gate.rs`) waits while `agents.rules.askBeforeShipping` is on (`ShipReason::Agent`) and,
whatever that rule says, when an agent chose the repo the run ships into (`ShipReason::AgentRepo`):
the run's own `repo`, or a project in `AppState.agent_set_repos`, the mark the Page view's Ask
first keeps. `agent_run` reads the repo and its mark in one `Session::repo_turn`, so no save lands
between the two.

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

The Page view shows the project's site, or an HTML page concept, with the models live in it. The
code below lives in `crates/maquette-preview`; core re-exports it as `site::preview` and `page`.

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
- **Ask first.** `commands/site/gate.rs` is the only way to a plan: `dev_plan` is private to
  `commands/site.rs`, so `start_page_preview`, `page_server_start` and `site_preview` all go through
  `gate::check`. A project in `AppState.agent_set_repos` needs its question in
  `AppState.preview_approvals`. `create_project`, `set_ship_settings` and `set_game_settings` write
  the mark through `RepoSetBy` before the repo, under `Session::repo_turn` with the write and any clear, so a failure or a race asks; an agent clearing a repo
  keeps it. A person's New project clears it, and a person's save only when the repo really moved:
  Settings and onboarding save the same repo as they click through.
  `PreviewAsk` (`maquette-preview`, `ask.rs`) is the question: the real folder, the command line, the
  the card's reading of it, and a hash of each of `package.json`, `.npmrc`, `pnpm-workspace.yaml`,
  `.yarnrc.yml`, `bunfig.toml` and `maquette.site.json` that exists. The key never depends on reading the
  command: it is the command line and those hashes, whatever the program, so a changed folder, command,
  `package.json` or listed manager config asks again. Reading only feeds the card, for exact forms with no
  flags (or the script Maquette picked itself); anything else says Maquette cannot tell what it runs.
  A yes trusts the repo, including nested packages and the files scripts load.
  This stops a dev command starting unseen and is not a sandbox: `add_built_asset` runs an agent's
  script in Blender on purpose.
  `Hub::request_preview` raises one `ApprovalKind::PagePreview` card per project and a thread waits: a
  yes is saved and the start goes through `check` again, so only what the person read runs; a no holds
  in memory for that question until a person asks from the app or the app restarts. A click in the app
  shows the same card, since the Page view starts its server just by opening.
- **Git on a repo.** `git status` runs what a repo's config says (a clean filter, for one), and no
  flag turns that off by name, so `commands/game.rs` only runs it in a repo a person chose, never one
  an agent named or set; there `uncommitted` is empty. `detect::uncommitted` also turns off
  `core.fsmonitor`, the untracked cache and submodules.
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

## Planning canvases

`maquette_core::plans` owns them: one plain JSON file per canvas at
`<project>/plans/<id>.plan.json`, the id being the file name and fixed for its life (Rename only
changes `name`). The file carries `version` (2; a newer one is refused, never rewritten), `name`,
`preset`, `created`, `edited`, `nodes` and `edges`. Nodes are `note`, `image`, `link`, `asset` or
`direction` with whole pixel `x` and `y`, an optional `width`, `title`, `text`, a project relative
`image`, an http(s) `url`, an `asset` id or a Lab `direction` id, and `by` (who added it, empty for
a person). Edges join two node ids with optional sides, a `kind` (`relates`, `leads-to`, `source`)
and a `label`. Pan and zoom are not stored, so two computers syncing the folder never fight over
the view. Names, titles, addresses and labels are trimmed of one set of white space on both sides
(`plans/text.rs`, mirrored by `state/plan-text.ts`, a test keeps them equal).

- **Versions.** `migrate.rs` reads the version first and lifts an older file step by step before
  it is parsed: version 1 gains `asset` and `direction` (null) on every node. Opening never writes;
  the file stays version 1 on disk until an edit or a rename saves it as version 2. A version 1
  file holding a kind it could not have is refused, and a file from a newer Maquette lists with a
  problem and is never saved over.
- **Presets.** `preset` (`brainstorm`, `story` or `mechanics`) only sets what a canvas starts
  with; it is not a node kind and every canvas holds every kind of node. `plans/presets.rs` lays
  out the starter notes: Brainstorm is blank, Story puts four beats left to right joined by
  `leads-to` links with two characters above and two places below (worded for a player in a game
  project and a visitor in a website, from the project's target), Game mechanics a card per system
  with its verb, look and input lines and a note on keeping systems apart. `create_seeded` (the
  app's New canvas and `create_canvas`) writes them through the usual checks, normalisation and
  size cap as the canvas's first log line, an add for each node and link under the person or the
  agent that made it, so one undo takes them all back. That line has no run; `create_canvas`
  answers with its change id for `undo_run`. A blank canvas writes no line and its answer has no
  `change`, as there is nothing to undo. When the line cannot be logged the new canvas is removed
  again (retrying while Windows holds the file, `base::fs::remove_retrying`), and a torn first
  line left in its log goes too, so the slug stays free for the next try. When the canvas still
  cannot be removed, the line is appended once more, so no starter is left without an undo. The field was there since version 2, so a
  seeded canvas stays version 2. The view's New canvas dialog and the empty list pick the preset
  (`features/plan/PresetPicker`, names in `state/plan-presets.ts`), the list shows it on each
  card and row, and mock mode keeps a copy of the seeds (`ipc/mock/plan-presets.ts`). `presets/seeds.json` holds
  the core's seeds as data, kept equal to them by a Rust test (which rewrites the file only when
  `MAQUETTE_BLESS_SEEDS` is `1`), and the mock's test compares its
  seeds with it card for card: order, places, text and links.
- **Links.** An `asset` or `direction` node keeps only the id: no name, picture, version or state.
  `maquette_core::plan_links` reads them live each time the canvas asks (`plan_links`): an asset's
  name, kind, stage, thumbnail, newest version, numbers and Inbox models to check, and a
  direction's images, picks and running Lab jobs. An id that names nothing reads as gone, which
  the node shows as not found; the file is never touched. The Inbox (which checks every model in
  it) is read only when the canvas links an asset, and a direction counts as running only while
  one of its images is being made, not while jobs are queued, held or waiting. The web side reads
  again when the canvas opens, its links change, the window comes back, every few seconds while a
  linked direction runs, and once after the project or Inbox settles from a burst of changes
  (`state/use-plan-links-store.ts`); `state/plan-links.ts` turns a read into the node's status and
  next step (Inspector, Inbox, Lab or Board).

- **Writes.** `files.rs` validates (`check.rs`: fields by kind, limits, paths safe on Windows),
  sorts nodes and edges by id and writes atomically under one lock, reading the file first; `edited`
  only moves when the content changes, so a save with nothing new writes the same bytes. A save
  keeps the file's name (only Rename changes it) and is refused for a removed canvas. A file over
  8 MB lists as broken without being read.
- **Changes, not snapshots.** A person's save, an agent's tool and an undo all go through
  `plans::change` (`change.rs`): it takes the lock, reads the file as it is now, lays a list of
  operations over it and writes the result with the same checks and size cap, then appends one line
  to the change log. The operations (`ops.rs`) are by node and edge id: add, remove, or change fields
  with the value before and after. `diff` finds the operations between two canvases, `apply` lays
  them over a third field by field, and `invert` turns a list around, which is all an undo is. So
  two writers never write over each other: each sends what it changed, and the other's work stays.
  The web side's `plan_apply` (it replaced the whole canvas save) sends the operations between the
  canvas the view last had from the file and what it shows, and returns what the file now holds.
- **Agents.** `plans/agent.rs` is what the general canvas tools call (`list_canvases`, `get_canvas`,
  `get_changes` read; `create_canvas`, `add_nodes`, `update_nodes`, `remove_nodes`, `layout_canvas`
  and `undo_run` write; `propose_removal` and `start_from_node` are under Extras below). Each write names the agent as `by` (the `agent` argument, else the MCP
  client's name, one line, at most 64 characters) and may name a `run` to group calls. At most 100
  nodes and links per call, and everything is checked before anything is written. An agent may add
  anything and edit or move any node, a person's too, since the log keeps what it said before. It
  never removes a person's node or link, an agent node a person wrote in, nor a node that has a
  link someone else drew: each is refused with a reason. Whether a person wrote in a node is in the
  file: an agent node whose content (kind, title, text, picture, address, asset or direction) a
  person wrote carries `personWrote: true`. A node an agent made that a person dragged carries `personPlaced: true`, written only when true like `personWrote`; auto layout and `layout_canvas` never move it, and `layout_canvas` naming it is refused. The agent may still move it with an explicit `x` and `y`. The core sets `personWrote` when a person writes the content (a
  move does not count) and no agent write clears it, so every agent change on top of the person's
  words stays a keeper. It is written only when true, so files without it read as before and stay
  version 2. An older app ignores it and drops it on its next save there; after that an agent may
  remove that node, and its later changes to it are no longer keepers, so the person's words in it
  can rotate out of the log. Agent names are labels, not accounts (any client can pass any name), so a
  node under another agent's name is refused too as a courtesy, but the only protection is between
  the person and agents. `place.rs` puts a node without `x` and
  `y` in a column right of the `near` node (or right of everything), clear of every other node by a
  gap, never stacked at 0, 0, with node sizes guessed as the view guesses them before a node has
  drawn. `layout_canvas` moves only the agent's own nodes that overlap another, to a free spot beside
  the node they link to. Nodes already clear and a person's nodes stay.
- **Change log.** Each canvas has `plans/<id>.log.jsonl`, one JSON line per write (`log.rs`): id, time,
  `by` (null for a person), `run`, a one line summary (`summary.rs`), the operations as written and the
  lines an undo took back. It is only appended to, so a sync tool sends the new lines and the canvas
  file stays small. An agent's line that changed a person's content (their node or link, or an agent
  node with `personWrote`) is a keeper (`log/keep.rs`): it is the only place their earlier version
  lives, so rotation never drops it until it is released. An undo releases what it names, the
  person's or the agent's own `undo_run`, and writes its line even when everything was changed since
  (no operations, "Nothing to undo, already changed", and the view says "Already changed, nothing to
  undo"). An undo merges carefully, so a field a later agent change rewrote would stay as that
  change left it while the undo released the only copy of the person's words: an undo is refused
  while a later agent change, not undone, touches the same field of the same node (or the same node
  or link as a whole) and is not part of the same undo ("Undo the later change first, or both
  together"). A run with both lines undoes them newest first. An undo line can not be undone itself,
  by the person or an agent ("an undo can not be undone"; the canvas's own Redo covers the person's
  session edits, and the Changes panel offers no Undo on undo lines): a redo would write the agent's
  words back with the person's only in lines that are not keepers. Undo lines therefore only ever
  write back a value from after the change being undone, so they do not block it. Held fields can
  then only be the person's own later writes, so releasing them loses nothing. A Keep must name
  every earlier agent change still held under the changes it keeps: on the same fields of the same
  node or link, and under those in turn, each on its own fields, so the chain can reach other nodes
  through a change that touched several (`keep_needs`, `plan_keep_needs`). The core refuses a Keep
  whose list leaves one out and names the missing ids; the app first shows "Keep the earlier changes
  too?" listing each one's summary and the parts it changed, with Keep all or Cancel, and keeps
  nothing the person did not confirm. So a kept change rotating out never leaves an older one whose
  undo would release the person's words. A change the person kept is theirs: an agent's undo of it
  (by id or in a run) is refused, and the person can still undo it from the Changes panel. Node and
  link ids are keyed by kind when undo order compares them. A kind change round-trips through undo like every other field. The keeper sizes
  count a line as a read of the log does, without the break that closes a torn line before it. The person's Keep and Keep run in the Changes panel (`plan_keep`) write a line with `keeps`
  that releases them without changing the canvas. Undo and keep lines are never keepers. An agent
  write is refused before anything is written when its line would pass 256 KiB (one node's longest
  text, before and after, fits) or when it changes a person's content and the keepers, less what it
  releases, would pass 4 MiB; an undo is never refused for room, and adding nodes still works. The
  keepers' sizes and the other lines' total are indexed in memory per log, so an append neither
  rereads the log for the budget nor to decide on rotation; the index is read again whenever the
  file's size or time is not what the last write left, or the file grew by more than the line just
  appended (another app appended meanwhile). Every
  other line (a person's own edits, agents' changes to their own nodes) stays undoable while it is
  among the newest: once those lines pass 1 MiB one atomic rewrite keeps every keeper and the newest
  512 KiB of the rest. A person's own line over 256 KiB keeps its summary but drops its operations
  (`trimmed`); the canvas holds what it wrote. The Changes panel's footer says the same. A line that
  does not read (a write cut short, a sync conflict copy) is skipped, and the next line starts on its
  own line. Removing a canvas moves its log to the trash first and the canvas after, putting the log
  back if the canvas cannot move, so a new canvas with the same id never inherits an old log.
- **Undo.** `plans::undo` inverts the operations of the lines named and lays them over the canvas as
  it is now, carefully: a field changed since stays as it was left, and a node edited or linked since
  is kept with its links and counted, so an undo never drops a link a person drew. An agent may undo only lines under its own name (`undo_run`, by `run` or by change ids, at most
  100 per call);
  a person's undo (`plan_undo`, the Changes panel's Undo and Undo run) may take back any. An undo is
  itself a logged change. The view's own Mod+Z, Shift+Mod+Z and Mod+Y undo and redo a person's edits
  in the open canvas (`state/plan-history.ts`, 100 edits deep) the same way, as inverted operations,
  so an agent's write that landed in between is not undone with them.
- **Extras.** Four additions, each a logged change the person can undo, with the person's side in
  `commands/plan_extras.rs` and the web side in `state/use-plan-extras-store.ts`:
  - _Removal proposals_ (`plans/removal.rs`). `propose_removal` asks the person to remove nodes an
    agent may not remove. It writes no operations: the ask is a log line with `proposes` (each node's
    id, name and a key of its content without its place, and the reason), so plain canvases need no
    new version. Rotation keeps an open ask, outside the keepers' budget, and at most 50 wait per
    canvas, 20 under one agent name. `removal::open` lists them with each node `here`, `changed` or `gone`, and the ask is
    stale when any is not `here`. Only the person answers (`plan_answer_removal`): Remove writes the
    person's own change (the nodes and their links, the card saying how many links go, drawn since
    the ask too), refused when stale or when the count of links changed since the person saw it, and Keep it writes a line with no operations; both carry
    `resolves`. Like a save, every answer and setting from the app names the opened canvas's
    `created` (`same_canvas`), so it never lands on another canvas that took the id. No MCP tool answers an ask, and `undo_run` refuses it.
    The Changes panel shows open asks first (`features/plan/RemovalAsks`), the nodes carry an "asked
    to remove" badge, and the footer says agents never delete a person's nodes, they ask.
  - _Start from a node_ (`plans/start.rs`). `start_from` copies the node and the nodes one link away
    (with the links among them, fresh ids, the node at 0, 0, authorship kept) into a new canvas
    through the usual create path, as its first line, under the person or the agent. The source
    never changes. Asset and direction nodes stay links, a stop never starts one and linked stops
    are left out, the preset follows the source (a brainstorm from the camera plan), and the camera
    plan's id stays reserved. The view offers it over the selected node (`NodeToolbar`).
  - _Auto layout after agents._ `autoLayout` on the canvas, on unless the person turns it off
    (`settings.rs`, a `set-auto-layout` operation). While on, `update_nodes` moves the agent's edited
    nodes the call gave no x or y that now overlap another clear, inside the same change, so the
    write stays one line with one undo (`autoLayout.moved`). A position the agent gives is kept, and
    a person's nodes, or an agent's node a person wrote in or moved, never move (here or in `layout_canvas`).
    `add_nodes` already places nodes without x and y clear. While off, nothing is tidied and
    `layout_canvas` is refused.
  - _Overlaps_ (`plans/overlap.rs`). On a Game mechanics canvas, two note cards whose `Verb:`,
    `Look:` or `Input:` lines share a main word overlap: the first content word of a verb, the last
    of a look, each key of an input with the gesture left out, lower case, plurals read as singular,
    starter wording left out. `PlanDoc.overlaps` carries them on every read and `get_canvas` shows
    them to agents. Mark as fine (`plan_mark_fine`, the person's only) stores `fine` marks keyed by an
    FNV-1a hash of both cards' titles and texts, so a flag comes back once either card's words
    change. `overlap/cases.json` holds the cases the core and the mock's copy (`ipc/mock/plan-overlap.ts`)
    are both tested on. The view flags the cards and lists the pairs in the Overlaps panel
    (`features/plan/OverlapsPanel`). The button keeps a count while any overlap exists: the ones to
    look at, or (muted) the ones marked as fine when every one is. The checks are cached per change
    of the cards' words.

  `autoLayout` (written only when off) and `fine` (only when not empty) make a file version 4; a
  file that uses neither stays version 2 (3 with a camera plan), so an older app still opens it, and
  one that does is refused by an older app rather than having them dropped.

- **Watching.** `commands/plan_watch.rs` watches the project's `plans/` folder while a canvas is open
  (`plan_watch`; one watcher per folder, shared by every window, the four newest folders stay; a
  project with no `plans/` folder yet answers false and the view asks again once it has a canvas)
  and emits `PlanChanged` once per canvas id after its file or log has been quiet for 250 ms, so a
  write by an agent, another window or a sync from another computer reaches the view. MCP writes also
  call `Host::plan_changed`, so the app does not wait for the watcher.
- **Person, agent and outside edits.** `state/plans-sync.ts` keeps the canvas as the file last held
  it (the base). A save sends the operations between the base and the flow; the answer becomes the
  new base, with edits made while it saved laid over it again. Its own writes read back as the base,
  so they never reload in a loop. When the file changes from outside and the person has no unsaved
  edits, it simply shows. When they do, the two are merged by node id (`state/plan-ops.ts`): the
  person's edits are laid over the new file, and only when both changed the same field of the same
  node does the canvas ask Keep mine or Use theirs. Until the person answers nothing saves: not
  autosave, leaving, quitting, renaming, opening a link (the link opens, unsaved) or an undo from
  the log, since any save would choose Keep mine for them; each says to answer first. Escape or a click outside
  closes the question without choosing; the top bar's save line then says to answer the clash and
  an Answer clash button asks again. A reload
  that a save overtook while it read the file reads again, and an outside change that lands mid
  drag keeps where the drag started for undo. When the folder cannot be watched the view says so
  and reads the canvas again each time the window comes to the front. An agent's
  write shows a toast, the nodes an outside write added or edited carry an "added" or "edited"
  badge, and an agent's nodes have a blue ring. The Changes panel (`features/plan/PlanChanges`,
  `state/plan-log.ts`) groups the log into runs and filters All, Agents or Mine.
- **A canvas replaced on disk.** When the open canvas's file is swapped for another under the same
  id (its `created` differs: removed and made again, by an agent or another computer), the view
  opens the new one afresh and says so when nothing was unsaved; with unsaved edits it saves
  nothing over the new file and says to copy what is needed and open it again from the list.
- **Rail, empty states and minimum size.** The rail lists the three canvases of the open project
  opened or edited last under Plan (`state/plan-recent.ts`, `state/use-plan-recent-store.ts`,
  `NavRail`'s `subs`): the list is read when the project opens and when a canvas it does not know
  changes or opens; a known canvas that changes is read alone (`plan_get`); the canvas open now
  takes its time from the Plan view's copy, so autosaves read nothing; and the Plan view's own
  list is taken when newer. When the person last had each canvas open is kept on this computer per
  workspace and project, and one edited since (an agent, a sync) carries a dot, never the canvas
  open now. Below 960 pixels of canvas width the minimap folds into a Map button under the zoom
  (`state/plan-view.ts`), and the first fit and Fit leave room for the add bar, the zoom and the
  map. The empty list and an empty canvas offer each way to start with its button and a prompt to
  copy for an agent (`features/plan/AgentPrompt`); the camera plan with no stops points at Add
  stop, and its panel says when a stop has no render yet. Everything works at 1120 by 720.
- **Leaving.** The web store waits for a save on its way and saves the latest edits before Back,
  another canvas, another project or quitting. When that save fails the canvas stays open and asks:
  try again, stay, or leave without saving; that dialog is the only message, no notice repeats it.
- **Pictures.** `images.rs` links a picture already inside the project by its relative path and
  copies one from elsewhere into `plans/images/<name>-<hash>.<ext>` once.
- **Remove.** The file moves to `<workspace>/trash/<project>/plans/`; copied pictures stay.
- **Edges.** The Tauri commands are in `commands/plans.rs` (`plan_apply`, `plan_log` and `plan_undo`
  beside list, get, create, rename and remove). The web state is
  `apps/web/src/state/plans.ts` (file to React Flow and back), `use-plans-store.ts` (saves a
  moment after each change) and `plans-sync.ts`, the view is `features/plan` on `@xyflow/react`, and mock mode serves
  demo canvases with `?plans=demo`.
- **Camera plan.** A website project has one built in canvas, `plans/camera-plan.plan.json`, pinned
  in the Plan list and never a preset (`plans/camera.rs` in maquette-model). Its nodes are `stop`
  nodes, each holding a whole `layout.json` stop, joined by `leads-to` links in tour order; the file
  also keeps `camera`, the stops it was based on. Only a file that uses either is version 3, so plain
  canvases stay version 2 and an older app still opens them; a newer version is never written over.
  `ensure` makes it from `layout.json` the first time it is opened, as the person's line. The id
  `camera-plan` is kept for it: a new canvas named "Camera plan" gets the next free suffix, and a
  canvas file from before that sits on the id is moved to a free id with its log (and a log line
  saying so) instead of blocking the camera plan. A log left on the id with no canvas is kept and
  the plan made again onto it when any line shows the camera plan (made as it, an Apply or rebase,
  its base, a stop node): its file deleted by hand, or a sync bringing the log first. A log that
  clearly belongs to another canvas (a remove whose moves all failed) is set aside to
  `plans/history/camera-plan-<stamp>-stray.log.jsonl` first, so the new plan never inherits it. The main
  line, the one Apply writes, is the chain with the most person stops, then the longest, then the
  leftmost; other chains are agents' proposals or stops set aside. Edits in the canvas and its side
  panel (`features/plan/CameraPanel`) save to the plan file only.
  `review` diffs the main line against `layout.json` as it is now (`camera/diff.rs`, a longest common
  run by stop id, so a moved stop shows where it leaves and where it lands, beside added, removed and
  changed settings) and returns two keys: a fingerprint of the whole `layout.json` and one of the
  plan. `apply` takes them back and goes through `layout::update` (its lock, checks and atomic
  write): it refuses, writing nothing, when either changed since the review was shown, keeps the old
  file as `plans/history/<change>.layout.json`, moves the plan's base and logs an Apply line as the
  person's. It is all or nothing: the plan and its log line are checked (validity, file size, a
  line short enough to keep its operations) before `layout.json` is written, and when the plan or
  line still fails to write, `layout.json` is put back from the kept copy and the copy removed. An
  undo reads the kept copy only from inside `plans/history/`. The line records `layout.json`'s key
  after the Apply as the file is stored (`layout::stored`: current version, clips tidied), so the
  undo still matches when the file had been edited by hand before the Apply. Its undo (`undo_apply`, Undo on that line in the Changes panel) puts the old file back
  only while `layout.json` is still what the Apply wrote; the general undo refuses Apply lines. When
  `layout.json` changes underneath, the view says the plan is based on an older one and offers
  Compare (a dry `rebase`), Rebase (takes outside changes the person did not also make, reports the
  clashes) and Start over (replaces only the main line; proposals and stops set aside stay); Apply
  waits until one is chosen, and neither side is overwritten silently. The view and the dry rebase
  carry `layoutKey`, and Rebase or Start over sends back the key of what the person saw; when
  `layout.json` changed again since, nothing is rebased and the comparison says to review again.
  Each is one person's change,
  refused with nothing written when its undo line would be too long to keep its operations.
  The facade `plan_camera.rs` in maquette-core adds each stop's poster render; the commands are in
  `commands/plan_camera.rs`. Agents have three tools of their own (`mcp/tools/plan_camera.rs`):
  `get_camera_plan`, `propose_camera_plan` (`camera/propose.rs`: the agent's own line below the
  main line, replacing its last proposal but keeping stops a person wrote in or linked) and
  `get_apply_preview` (both reads write nothing, and answer from `layout.json` when there is no
  plan yet), and `update_nodes` edits a stop's settings. No MCP route applies a plan or
  undoes an Apply.

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
4. Backend work goes in `maquette-core`; the desktop crate only adds the command that calls it. A
   new IPC type goes into the `register` of the module that defines it.
5. Run `pnpm gate`.
