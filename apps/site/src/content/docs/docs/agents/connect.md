---
title: Connect an agent
description: Turn on the local MCP server in Settings and add it to Claude Code or Codex, so an agent can read the Board and run the pipeline.
sidebar:
  label: Connect an agent
---

Settings > Agents runs a Model Context Protocol server inside the app, so Claude Code and Codex can drive Maquette. The docs explain what Maquette does. The tools act, on this machine, with your approval for anything that spends or ships.

## What the server is

- Off by default.
- Listens on `127.0.0.1` only, at `/mcp`, over Streamable HTTP. The port is 4791 unless you change it.
- Wants `Authorization: Bearer <token>`, where `<token>` stands for the real token: never paste the placeholder. The token is made the first time the server starts and kept in the app state on this machine, never in the workspace. New token replaces it.

## Before you connect

The server lives inside the Maquette app, so the app has to be open with the server on. Close the app and the port closes with it: an agent that tries then gets `ECONNREFUSED` (connection refused), or its Maquette tools show as failed.

1. Open Maquette and go to Settings > Agents.
2. Turn the server on. The line under it says "Listening on http://127.0.0.1:4791/mcp" when it works, or why it could not start (usually the port is taken: change it there).
3. Copy the token from the same place.
4. Start your agent after that. If the agent was already running, reconnect it: in Claude Code run `/mcp` and choose Maquette, or restart the session.

The setting is kept, so the server starts by itself the next time you open the app. You still have to open the app first.

## Add it to your agent

Copy config in Settings > Agents gives a working setup with your token filled in.

For Claude Code:

```sh
claude mcp add --transport http maquette http://127.0.0.1:4791/mcp --header "Authorization: Bearer <token>"
```

For Codex, in `~/.codex/config.toml`:

```toml
[mcp_servers.maquette]
url = "http://127.0.0.1:4791/mcp"
http_headers = { Authorization = "Bearer <token>" }
tool_timeout_sec = 150
```

On Windows, the same `claude mcp add` line works in PowerShell. Then ask the agent to call `list_projects`. Every tool that works on a project takes its id as `project`. None of them changes which project the app has open.

## The first call

An agent starts with `get_started`. It works in a brand new setup and reads the workspace, the tools, whether each key is saved (true or false, never the value), every project with its stage, the gates only you can pass and a `next` list of what to do now. With no project yet, `create_project` makes one; it spends nothing and needs no approval.

Everything still works without API keys: By hand sends and built assets cost nothing. `get_started` and the errors say so when a key, the Codex sign in or Blender is missing, because only you can fix those.

## What an agent cannot do

These rules hold in the app. They are not a sandbox: see the last point.

- Blender only runs headless. No tool opens its window.
- There is no screenshot or screen capture tool.
- A send to Tripo or Meshy always waits for you to Approve in the app before any credit is spent, whatever other settings say. Deny cancels the job.
- A round of Lab images on OpenAI Images that an agent asks for waits for you to Approve in the app, with what it should cost, before anything is sent. Deny cancels its jobs. Codex rounds and rounds you start in the Lab do not ask.
- An agent cannot change the monthly OpenAI image cap. `set_image_backend` changes the backends and quality only; the cap is yours, in Settings.
- With Ask before shipping on (the default), a run started over MCP stops before its Ship step and the app shows Approve or Deny.
- A repo an agent names (`create_project`, `set_ship_settings`, `set_game_settings`, the `repo` of a run) must be a git repo or worktree given as an absolute path or with `~`. The workspace, anything inside or around it, the home folder and any folder holding it are refused, the real folder behind a link is what gets saved, and a repo does not change while its project has a run going.
- The first start of the dev server of a repo an agent set waits for you. The card names the folder and the command. For a plain `npm run dev`, `pnpm dev` or the like it also shows what the script runs; for any other command it says Maquette cannot tell what it runs. Approve runs it and is remembered. A changed folder, command, `package.json` or listed manager config (`.npmrc`, `pnpm-workspace.yaml`, `.yarnrc.yml`, `bunfig.toml`, `maquette.site.json`) asks again. Deny leaves the server stopped and `start_page_preview` answers an error with code `preview-refused`. This holds for the Page view and Preview on site too. A repo you set yourself in the app never asks, and saving the same repo again keeps the question; pick another folder to clear it.
- A yes trusts the repo, including nested packages and the files scripts load: those can change without asking.
- Settings overrides are checked by name. An unknown one is refused with the list of valid names.
- The preview approval stops a dev command starting unseen. It is not a sandbox: `add_built_asset` runs an agent's Python in headless Blender on purpose, with your permissions.

Every call shows in the activity list under Settings > Agents.

## If it does not connect

`ECONNREFUSED` means nothing is listening on that port on this computer. It is not a token problem and not a network problem: the app is closed, the server is off, or the port differs from your config. A wrong token gets through to the server and comes back as 401 instead. To tell them apart without an agent:

```sh
curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:4791/mcp
```

`000` means nothing is listening (see `ECONNREFUSED` below). `401` means the server is on and only wants the token.

| What you see                                      | What to do                                                                                                                    |
| ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `ECONNREFUSED`, or the server is listed as failed | Maquette is not open, or the server is off or on another port. Open it, turn the server on, reconnect.                        |
| 401 Unauthorized                                  | The token is old or wrong. Copy it again from Settings > Agents. New token replaces the old one.                              |
| The line under the toggle shows an error          | Usually another program holds the port. Pick another port in Settings > Agents and update the agent's config.                 |
| Plugin installed, tools still failed              | `MAQUETTE_MCP_TOKEN` was not set in the shell that started Claude Code. See [Agent skills](/docs/agents/skills/#claude-code). |

Next: [MCP tools](/docs/agents/mcp-tools/), then the [agent skills](/docs/agents/skills/) that teach an agent the workflows.
