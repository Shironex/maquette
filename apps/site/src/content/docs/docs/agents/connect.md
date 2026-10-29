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

Then ask the agent to call `list_projects`. Every tool that works on a project takes its id as `project`. None of them changes which project the app has open.

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

Next: [MCP tools](/docs/agents/mcp-tools/).
