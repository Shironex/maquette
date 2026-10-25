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

## Adding a feature

1. Create `apps/web/src/features/<feature>/` with one folder per component, each with its test and
   `index.ts`, and put state in the `.hooks.ts` beside it.
2. Import only shared features and non feature modules. If the feature must render another feature,
   let the shell (or another composition root) pass it in. If it becomes something every feature
   needs, add it to `SHARED_FEATURES`; if it assembles other features, to `COMPOSITION_ROOTS`.
3. Give the shell its view (`src/state/views.ts` and `CurrentView` in the shell).
4. Backend work goes in `maquette-core`; the desktop crate only adds the command that calls it.
5. Run `pnpm gate`.
