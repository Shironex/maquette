---
title: Install
description: Get Maquette running and pass the toolchain check. Blender is the only tool it cannot work without.
sidebar:
  label: Install
---

Maquette is a desktop app for macOS and Windows, built with Tauri 2. It is early, so expect bugs and back up your workspace before you update.

## Install a release

Open the [latest release](https://github.com/Shironex/maquette/releases/latest) and download the file for your computer. The version is in the file name.

| System               | File                               |
| -------------------- | ---------------------------------- |
| macOS, Apple silicon | `Maquette_<version>_aarch64.dmg`   |
| Windows, x64         | `Maquette_<version>_x64-setup.exe` |

There is no build for Intel Macs, Windows on ARM or Linux yet.

Open the `.dmg` and drag Maquette to Applications, or run the setup file. Do not run Maquette from inside the `.dmg`: drag it out first.

### The first launch

The builds are not signed by Apple or Microsoft yet, so your system asks once.

- **macOS.** Open Maquette from Applications. If it says the app cannot be opened or could not be verified, close that message, open System Settings > Privacy & Security, scroll to the message about Maquette and choose Open Anyway, then confirm with your password. If the message never offers it, run `xattr -dr com.apple.quarantine /Applications/Maquette.app` in Terminal and open the app again.
- **Windows.** If SmartScreen stops it ("Windows protected your PC"), choose More info, then Run anyway.

Updates that come through the app do not ask again. See [Updates](/docs/start/updates/).

### The Keychain prompt on a Mac

Because the app is not signed with a developer certificate, macOS asks again when Maquette reads a provider key it stored earlier: "Maquette wants to use your confidential information stored in "com.shironex.maquette" in your keychain." It shows when you open Settings or send to a provider, and again after each update, since an update is a new build to macOS. It is Maquette reading its own keys, nothing else. Enter your Mac login password and choose Always Allow, which is the normal answer; Allow only lasts for that one read. It never shows when you have saved no key.

## Run it from source

For contributors, or to try the latest `main`. You need Node 22.12 or newer, pnpm 10 and the Rust toolchain pinned in `rust-toolchain.toml`.

```sh
pnpm install
pnpm dev        # the desktop app with hot reload
```

`pnpm dev:web` runs the UI alone in a browser, in mock mode, with no real tools behind it.

## The toolchain

Maquette calls tools you already have and checks for them on first run. It looks for six. On a clean machine, install Blender first; the rest depend on what you want to do.

| Tool                 | What Maquette uses it for                                                                 | Install                                                                                                                          |
| -------------------- | ----------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| Blender              | Every build, bake, export, poster and thumbnail. Always headless. The only required tool. | [blender.org](https://www.blender.org/download/), or `brew install --cask blender` or `winget install BlenderFoundation.Blender` |
| Codex CLI            | The Concept lab's image jobs. Run `codex login` once after installing it.                 | `npm i -g @openai/codex`, then `codex login`                                                                                     |
| ffmpeg               | The Lab's contact sheet, and GIF and MP4 render loops.                                    | `brew install ffmpeg` or `winget install Gyan.FFmpeg`                                                                            |
| KTX-Software `toktx` | KTX2 textures in Compress. Without it, textures stay PNG or JPEG.                         | The installer on the [KTX-Software releases](https://github.com/KhronosGroup/KTX-Software/releases) page                         |
| gltf-transform       | The Compress step.                                                                        | `npm i -g @gltf-transform/cli`, or nothing when Node is there (it runs through `npx`)                                            |
| Node                 | The Ship step, which runs `tools/ship.mjs`, and a game's export and ship scripts.         | [nodejs.org](https://nodejs.org/), 22 or newer                                                                                   |

The check runs `--version` on each tool. Locate a tool by hand if it is not on your PATH, and recheck.

**Blender.** Maquette has no minimum version check. It is built and tested against Blender 4.x and 5.x, so use the current stable release or a 4.2 LTS or newer. It finds Blender in the usual places without a PATH entry: `/Applications` on a Mac, and `C:\Program Files\Blender Foundation\Blender <version>` on Windows (the newest when several are installed). Without Blender the app opens, but new downloads are held and nothing builds; the check says so.

**What you can do without the rest.** With only Blender you can import models, run the Inspector, fix-ups, Parts, Motion and Render styles, and use the By hand route (below) for new models. Codex CLI (or an OpenAI API key) is only for generating concept images in the Lab. Tripo or Meshy API keys are only for sending refs to them from the app.

## First run

After a welcome screen, the onboarding walks through six steps.

1. A workspace folder. Projects, the inbox and the cache live under one root. Pick an empty folder, or an existing Maquette workspace.
2. The toolchain check above.
3. Defaults for every new model and build, such as the budgets and the compression settings. A project can override them later.
4. The agents step, which can start the local MCP server. It is off by default. See [Connect an agent](/docs/agents/connect/).
5. Your first project: a template, a name and an optional site repo, or an imported folder.
6. A summary of what was set up.

The workspace is a plain folder you picked, for example `~/Maquette` or `Documents\Maquette`. Nothing is created anywhere else except the app's own settings and the keys below.

### The By hand route, with no keys

You can make models without any provider key. In a project, open an asset at Refs, choose Send to 3D, then By hand. Maquette writes a folder with the images and a settings card and spends nothing. Make the model on the provider's website, export a GLB with the name the card gives, and save it to your Downloads folder. Maquette watches Downloads (Settings > 3D providers) and puts the file in the Inbox, where you check it and Accept it.

### Keys, when you want the API routes

- **Tripo and Meshy:** Settings > 3D providers, paste the key, Test key (free). Every send then waits for your Approve.
- **OpenAI Images:** Settings > Image generation, only if you want it instead of Codex for Lab images.
- **Codex:** no key in Maquette. Install the Codex CLI and run `codex login` in a terminal, then Recheck the toolchain.

## Where things live

- **Files:** the workspace folder. No account and no cloud project.
- **API keys:** the OS keychain (macOS Keychain on a Mac, Windows Credential Manager on Windows), under the service `com.shironex.maquette`. On a Mac, see [the Keychain prompt](#the-keychain-prompt-on-a-mac). Never in `maquette.json`, a project file or a log. Settings shows only the last four characters.
- **Processes:** every tool is spawned directly with an args array, never through a shell.

Next: [Your first project](/docs/start/first-project/).
