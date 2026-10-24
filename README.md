# Maquette

A desktop studio that turns AI concept art into real-time 3D web scenes. One board follows each asset through the pipeline: Concept, Refs, Tripo, Blender, Web, Ship. Everything runs on this machine.

This first slice covers the app shell, onboarding, the toolchain check, workspace config and Settings. The pipeline runner, the 3D inspector and the MCP server come later.

## Stack

- **Shell:** Tauri 2 (`apps/desktop`), macOS and Windows
- **UI:** React 19, Vite, Tailwind CSS v4, zustand (`apps/web`)
- **Kit:** typed components with a theme generated from `tokens.json`, Phosphor icons, Geist fonts bundled (`packages/ui`)
- **Core:** Rust domain logic (`crates/maquette-core`): tool detection and `--version` parsing, the `maquette.json` workspace config, projects and app state
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
- `?scenario=configured`: a workspace with Moon Island open
- `?scenario=no-blender`: Blender missing, so setup stops at the toolchain step
- `?delay=700`: how long a toolchain check takes, in milliseconds

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
