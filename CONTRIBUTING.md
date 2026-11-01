# Contributing

Thanks for looking. Maquette is a desktop studio (Tauri 2, Rust and React) that turns concept art
into optimised 3D assets. This file covers building it, the checks every change passes, and how
changes are proposed. If you only want to report something, the issue forms ask for exactly what is
needed to act.

## What is welcome

- **Bug reports and ideas:** open an issue with the matching form. Larger ideas start under Ideas
  in [Discussions](https://github.com/Shironex/maquette/discussions), so they can take shape first.
- **Code:** pull requests that fix a bug are welcome once there is an issue for it. A new feature
  starts as a discussion; please do not open a feature PR before the idea is agreed.
- **Not open to pull requests:** art, example models and the app's visual identity, and anything
  that changes how API keys, the agent token or the update path are handled. Open an issue first.

Changes you send are licensed under the project's license, AGPL-3.0.

## Setup

You need:

- Node 22.23.4 or newer (`.nvmrc` says the same) and pnpm 10 (`packageManager` in `package.json`).
- The Rust toolchain pinned in `rust-toolchain.toml`. With [rustup](https://rustup.rs) it installs
  itself the first time you run `cargo` in the repo.
- The [Tauri 2 system dependencies](https://tauri.app/start/prerequisites/) for your OS: Xcode
  Command Line Tools on macOS, the Visual Studio C++ build tools and WebView2 on Windows.

```sh
pnpm install
pnpm dev        # the desktop app with hot reload
pnpm dev:web    # the UI alone in a browser, in mock mode (no Rust, no Blender)
pnpm dev:site   # the landing page
```

Mock mode answers every command from an in-memory backend, so most UI work needs nothing else. The
README's Development section lists the `?scenario=` query parameters.

To run real pipelines you also need the tools Maquette drives. The app's toolchain check lists
which are missing and how to install them:

- [Blender](https://www.blender.org/download/), always run headless by the pipeline.
- `toktx` from [KTX-Software](https://github.com/KhronosGroup/KTX-Software/releases), for texture
  compression.
- `gltf-transform` (`@gltf-transform/cli`) for the Compress step, and `ffmpeg` for contact sheets
  and video.
- Optional: Tripo and Meshy API keys (kept in the OS keychain) and the Codex CLI for the Concept
  lab. You do not need any of them to work on code.

Rust tests that call a real Blender skip themselves when it is not installed; set
`MAQUETTE_TEST_BLENDER` to its binary to run them.

## Checks

```sh
pnpm gate
```

`pnpm gate` is the whole local gate and has to exit 0 before you open a pull request. It runs two
lanes side by side and ends with each step's time and what failed. The Node lane: ESLint, the repo's
own meta lint (`scripts/lint-meta.mjs`), typecheck, Vitest, the release scripts' tests
(`pnpm test:scripts`), the site build and Prettier. The Rust lane: `cargo fmt --check`, clippy with
warnings denied, and the tests. Vitest, clippy and the Rust tests each take every core, so they
take turns; the rest overlap. Husky runs lint-staged on commit and commitlint on the message.

The gate differs from CI in three ways:

- **Rust tests:** it uses [cargo-nextest](https://nexte.st) (`cargo install cargo-nextest --locked`)
  plus `cargo test --doc`. Without nextest it runs `cargo test` and says so; the tests are the same.
  CI runs `cargo test`, on both macOS and Windows.
- **Cache:** ESLint and Prettier keep a cache in `node_modules/.cache`, so a rule that looks at other
  files (a component's required test, say) may not re-check an unchanged file. CI has no cache.
  `pnpm gate --no-cache` runs without it.
- **Generated files:** the gate fails if the Rust tests rewrote the IPC bindings or `tool-names.txt`,
  because the Node lane may have read either version. Commit the new files and run it again.

`pnpm gate:fast` checks only what changed since `main`, for quick rounds while you work. It lints and
formats those files, runs the Vitest tests they affect, and skips the lane or steps they cannot reach;
a change it cannot place runs the whole gate. It proves less and does not replace `pnpm gate`.

Architecture rules (folder per component, one way dependencies, file size limits) are in
[docs/architecture.md](docs/architecture.md).

For any change you can see, also run:

```sh
pnpm visual
```

It renders mock mode in headless Chromium and writes a screenshot of every view to `.visual/`;
look at them. It needs a Playwright install that this repo does not carry: point `PLAYWRIGHT` at
any local copy (`PLAYWRIGHT=/path/to/playwright/index.mjs pnpm visual`). `ONLY=<set>` renders one
set alone, and the header of `scripts/visual-check.mjs` lists them. It renders pages to files and
never captures your screen.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/), always with a scope, one logical
change per commit, summary in lowercase:

```
fix(core): keep the lock file out of the sync ignore list
```

`commitlint.config.mjs` enforces it. The allowed scopes follow the folders:

| Scope       | What it covers                              |
| ----------- | ------------------------------------------- |
| `desktop`   | `apps/desktop`, the Tauri shell             |
| `web`       | `apps/web`, the React UI                    |
| `site`      | `apps/site`, the landing page               |
| `ui`        | `packages/ui`, the UI kit                   |
| `core`      | `crates/maquette-*`, the Rust domain logic  |
| `workspace` | workspace files and cross-cutting changes   |
| `deps`      | dependency bumps                            |
| `docs`      | documentation and `.github` community files |
| `scripts`   | `scripts/` and tooling                      |

Types are the usual ones: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `ci`, `build`, `perf`.

## Pull requests

1. Branch from `main` (`fix/short-name`, `feat/short-name`).
2. Keep the change focused and the commits small. Add or update tests with the code.
3. Run `pnpm gate` (and `pnpm visual` for UI) until both are clean.
4. Open the PR with a conventional title in the same shape as a commit; it becomes the merge
   commit. Fill in the template and link the issue with `Closes #N`.
5. The PR gets the labels of the issue it closes (type, priority and `area:*`).

Pull requests are merged with a merge commit, so every commit of the branch stays in the history.
Keep each one a small conventional commit; commitlint checks them as you commit.

## Reporting

Bugs and model or pipeline problems have their own forms, which ask for the version, your OS,
whether the workspace is synced between two computers, the Blender version and the log. Security
problems do not go in a public issue: see [SECURITY.md](SECURITY.md).
