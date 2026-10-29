---
title: Agent skills
description: Four skills that teach Claude Code and Codex the Maquette workflows, from concept rounds and by hand sends to character rig fixes and game export. Install them from the repo.
sidebar:
  label: Agent skills
---

The [MCP tools](/docs/agents/mcp-tools/) let an agent act. The skills tell it how to act well: which tool to call first, what to look at before it goes on, and where it must stop and wait for you. They are plain `SKILL.md` files, one folder per skill, in [`plugins/maquette/skills`](https://github.com/Shironex/maquette/tree/main/plugins/maquette/skills) of the repo. Claude Code and Codex read the same files.

## The skills

| Skill                    | Use it to                                                                                                                                                                                   |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `maquette-lab-round`     | Run a concept art round: read the Codex usage and image backend, set the style board, start a direction per asset, let you pick, make object refs and the missing views, clean backgrounds. |
| `maquette-by-hand-round` | Write By hand handoffs for Tripo or Meshy, then land the downloads: match them by picture, attach, read the inbox checks, accept, add to the library, redo a bad generation.                |
| `maquette-character-rig` | Take a rigged character from its rig check to a clean v2: pose test, fix skin weights per island, verify, add the file as the next version, bake clips.                                     |
| `maquette-game-export`   | Set up a game project, import its models, set budgets and export, plan and run the pipeline, wait for your approval at Ship and check the files in the repo.                                |

The rig skill carries headless Blender scripts (a humanoid set and a creature set with an example config). Everything else is instructions.

## Rules every skill keeps

- A person picks the concepts. The agent presents options and waits.
- Characters and creatures are generated without held weapons, shields or lanterns. Those are separate props, attached to hand bones after rigging.
- Sends to Tripo or Meshy and OpenAI image rounds an agent starts wait for your approval in the app. The agent never starts them again while one waits, and never changes the monthly cap.
- A download is matched by looking at its picture next to the waiting sends, never by a score alone.
- Ship waits for you. The agent polls the run and never starts it again. Ship never commits.
- Blender only runs headless and nothing captures your screen.

## Claude Code

The repo is a plugin marketplace. In Claude Code:

```sh
/plugin marketplace add Shironex/maquette
/plugin install maquette@maquette
```

The skills then appear as `/maquette:maquette-lab-round` and so on, and Claude picks them up from what you ask for.

The plugin also declares the Maquette MCP server, so the tools come with it. It reads the token and the port from your environment; set them in the shell you start Claude Code from. Copy the token from Settings > Agents:

```sh
export MAQUETTE_MCP_TOKEN="<token>"
export MAQUETTE_MCP_PORT=4791   # only when you changed the port
```

If you already added the server yourself as described in [Connect an agent](/docs/agents/connect/), keep one of the two entries so the tools are not listed twice.

## Codex

Codex finds skills in `$HOME/.agents/skills` and in a repository's `.agents/skills`, and follows symlinked folders. Clone the repo once and link the skills, so one source serves both agents and `git pull` updates them:

```sh
git clone https://github.com/Shironex/maquette.git
node maquette/plugins/maquette/install-codex.mjs
```

That links each skill into `~/.agents/skills`. Use `--repo <path>` to link into one repository instead, `--copy` to copy rather than link, and `--remove` to take them out again. On Windows the link is a directory junction, which needs no administrator rights. Restart Codex if a skill does not show up, then type `$` to mention one.

Add the server to `~/.codex/config.toml`. Codex reads the token from an environment variable of your choosing:

```toml
[mcp_servers.maquette]
url = "http://127.0.0.1:4791/mcp"
bearer_token_env_var = "MAQUETTE_MCP_TOKEN"
tool_timeout_sec = 150
```

## Keeping the skills honest

`pnpm lint:meta` (part of `pnpm gate`) runs `scripts/check-skills.mjs`. It reads the tool list the MCP handshake test expects and fails when a skill names a tool the server does not have, when a skill's name does not match its folder, when its description does not say when to use it, when a skill passes 250 lines, when a dash slips in, or when the plugin's MCP entry carries a literal token.

Next: [MCP tools](/docs/agents/mcp-tools/).
