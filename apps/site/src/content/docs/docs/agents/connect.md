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
- Wants `Authorization: Bearer <token>`. The token is made the first time the server starts and kept in the app state on this machine, never in the workspace. New token replaces it.

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

## What an agent cannot do

The agent rules hold by construction.

- Blender only runs headless. No tool opens its window.
- There is no screenshot or screen capture tool.
- A send to Tripo or Meshy always waits for you to Approve in the app before any credit is spent, whatever other settings say. Deny cancels the job.
- A round of Lab images on OpenAI Images that an agent asks for waits for you to Approve in the app, with what it should cost, before anything is sent. Deny cancels its jobs. Codex rounds and rounds you start in the Lab do not ask.
- An agent cannot change the monthly OpenAI image cap. `set_image_backend` changes the backends and quality only; the cap is yours, in Settings.
- With Ask before shipping on (the default), a run started over MCP stops before its Ship step and the app shows Approve or Deny.
- Settings overrides are checked by name. An unknown one is refused with the list of valid names.

Every call shows in the activity list under Settings > Agents.

## If it does not connect

| What you see                                      | What to do                                                                                                                    |
| ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `ECONNREFUSED`, or the server is listed as failed | Maquette is not open, or the server is off or on another port. Open it, turn the server on, reconnect.                        |
| 401 Unauthorized                                  | The token is old or wrong. Copy it again from Settings > Agents. New token replaces the old one.                              |
| The line under the toggle shows an error          | Usually another program holds the port. Pick another port in Settings > Agents and update the agent's config.                 |
| Plugin installed, tools still failed              | `MAQUETTE_MCP_TOKEN` was not set in the shell that started Claude Code. See [Agent skills](/docs/agents/skills/#claude-code). |

Next: [MCP tools](/docs/agents/mcp-tools/), then the [agent skills](/docs/agents/skills/) that teach an agent the workflows.
