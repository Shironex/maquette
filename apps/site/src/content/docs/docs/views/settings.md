---
title: Settings
description: The workspace, the toolchain, the 3D providers, image generation, avatar defaults, budgets and compression, the agents, the open project, and About.
sidebar:
  label: Settings
---

Settings saves as you go. The nav on the left jumps to a section; a field that is not valid holds the save and asks before you leave.

| Section                 | What it holds                                                                    |
| ----------------------- | -------------------------------------------------------------------------------- |
| Workspace               | The root folder, its layout, the cache and how to share it with another computer |
| Toolchain               | The tools Maquette found, with Recheck and Locate                                |
| 3D providers            | API keys, the Send to 3D presets and the Downloads watch for By hand             |
| Image generation        | Codex or OpenAI Images per kind of Lab image, the OpenAI key, quality and cap    |
| Avatar defaults         | The author and licence every new VRM avatar starts from, and its material        |
| Budgets and compression | What a scene may weigh, and the compression every model starts from              |
| Agents                  | The local MCP server. See [Connect an agent](/docs/agents/connect/)              |
| About                   | The version you run, updates, and the links below                                |
| This project            | The open project's name and site repo                                            |

## Image generation

- **Backend per kind.** Concepts, Object refs, Other views and Clean background each run on Codex (the default) or OpenAI Images. See [Lab](/docs/views/lab/#openai-images).
- **OpenAI key.** Paste an API key from platform.openai.com, API keys. It is kept in the system keychain, never in a file, and only its last four characters show. Test key only lists the models the key can use, which costs nothing, and says whether it can use the image model.
- **Quality.** Low, medium or high, each with its estimate per image.
- **Monthly cap.** Whole US dollars the Image API may spend per calendar month, $10 by default; 0 stops it. Spent this month shows what went out since the first of the month, from `credits.jsonl`. A round that would pass the cap does not start. The check counts each image at what one has cost lately at this quality, from the ledger, so a round that fits is not refused on the price list's higher guess.

## Avatar defaults

What every new [Avatar](/docs/views/avatar/) starts from: the author, contact and copyright, the licence (who may play it, commercial use, credit, changes, the yes or no uses and an other licence URL) and the material, MToon soft or PBR. The licence starts at VRM 1.0's defaults, the strictest. An avatar that is already saved keeps its own; change it in the Avatar view.

## About

- **The build.** The version, the commit and the day it was built. A development build says "dev build" instead.
- **The machine.** The OS and its version, the architecture, the Tauri version and the Blender version from the last toolchain check.
- **Updates.** Whether this build updates itself and what the last check found, with Check now, the release notes, Download and Restart to update. See [Updates](/docs/start/updates/).
- **Links.** Release notes, these docs and the source on GitHub, opened in your browser.
- **Copy version info.** Puts a short block on the clipboard with the version, commit, OS, Tauri and Blender versions, made for pasting into a bug report.
- **Report a problem.** Opens the bug report form on GitHub with the version, OS and Blender fields filled in. You pick the rest and submit it yourself.

Maquette sends no usage data. About only fills in what you choose to paste or submit.
