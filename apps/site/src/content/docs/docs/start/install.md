---
title: Install
description: Get Maquette running and pass the toolchain check. Blender is the only tool it cannot work without.
sidebar:
  label: Install
---

Maquette is a desktop app for macOS and Windows, built with Tauri 2. The first release, v0.1.0, is out on [GitHub Releases](https://github.com/Shironex/maquette/releases/tag/v0.1.0). It is early, so expect bugs and back up your workspace before you update.

## Install a release

Download the file for your computer from the [release page](https://github.com/Shironex/maquette/releases/tag/v0.1.0).

| System               | File                           |
| -------------------- | ------------------------------ |
| macOS, Apple silicon | `Maquette_0.1.0_aarch64.dmg`   |
| Windows, x64         | `Maquette_0.1.0_x64-setup.exe` |

Open the `.dmg` and drag Maquette to Applications, or run the setup file.

### The first launch

The builds are not signed by Apple or Microsoft yet, so your system asks once.

- **macOS.** If it says the app cannot be opened, open System Settings > Privacy & Security, find the message about Maquette and choose Open Anyway.
- **Windows.** If SmartScreen stops it, choose More info, then Run anyway.

Updates that come through the app do not ask again. See [Updates](/docs/start/updates/).

## Run it from source

For contributors, or to try the latest `main`. You need Node 22.12 or newer, pnpm 10 and the Rust toolchain pinned in `rust-toolchain.toml`.

```sh
pnpm install
pnpm dev        # the desktop app with hot reload
```

`pnpm dev:web` runs the UI alone in a browser, in mock mode, with no real tools behind it.

## The toolchain

Maquette calls tools you already have and checks for them on first run. It looks for six.

| Tool                 | What Maquette uses it for                                                                 |
| -------------------- | ----------------------------------------------------------------------------------------- |
| Blender              | Every build, bake, export, poster and thumbnail. Always headless. The only required tool. |
| Codex CLI            | The Concept lab's image jobs.                                                             |
| ffmpeg               | The Lab's contact sheet.                                                                  |
| KTX-Software `toktx` | KTX2 textures in Compress. Without it, textures stay PNG or JPEG.                         |
| gltf-transform       | The Compress step.                                                                        |
| Node                 | The Ship step, which runs `tools/ship.mjs`.                                               |

The check runs `--version` on each tool. Locate a tool by hand if it is not on your PATH, and recheck.

## First run

After a welcome screen, the onboarding walks through six steps.

1. A workspace folder. Projects, the inbox and the cache live under one root. Pick an empty folder, or an existing Maquette workspace.
2. The toolchain check above.
3. Defaults for every new model and build, such as the budgets and the compression settings. A project can override them later.
4. The agents step, which can start the local MCP server. It is off by default. See [Connect an agent](/docs/agents/connect/).
5. Your first project: a template, a name and an optional site repo, or an imported folder.
6. A summary of what was set up.

## Where things live

- **Files:** the workspace folder. No account and no cloud project.
- **API keys:** the OS keychain (macOS Keychain or Windows Credential Manager), under the service `com.shironex.maquette`. Never in `maquette.json`, a project file or a log. Settings shows only the last four characters.
- **Processes:** every tool is spawned directly with an args array, never through a shell.

Next: [Your first project](/docs/start/first-project/).
