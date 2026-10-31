---
title: Agent skills
description: Five skills that teach Claude Code and Codex the Maquette workflows, from concept rounds and by hand sends to character rig fixes, VRM avatars and game export. Install them from the repo.
sidebar:
  label: Agent skills
---

The [MCP tools](/docs/agents/mcp-tools/) let an agent act. The skills tell it how to act well: which tool to call first, what to look at before it goes on, and where it must stop and wait for you. They are plain `SKILL.md` files, one folder per skill, in [`plugins/maquette/skills`](https://github.com/Shironex/maquette/tree/main/plugins/maquette/skills) of the repo. Claude Code and Codex read the same files.

## The skills

| Skill                    | Use it to                                                                                                                                                                                                                                     |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `maquette-lab-round`     | Run a concept art round: read the Codex usage and image backend, set the style board, start a direction per asset, let you pick, make object refs and the missing views, clean backgrounds.                                                   |
| `maquette-by-hand-round` | Write By hand handoffs for Tripo or Meshy, then land the downloads: match them by picture, attach, read the inbox checks, accept (one by one or many), compare two versions, make render pictures, add to the library, redo a bad generation. |
| `maquette-character-rig` | Take a rigged character from its rig check to a clean v2, through the rig doctor first: pose test, fix skin weights per island, verify, add the file as the next version, bake clips.                                                         |
| `maquette-game-export`   | Set up a game project, import its models, set budgets and export, plan and run the pipeline, wait for your approval at Ship and check the files in the repo, or write an export pack for another engine.                                      |
| `maquette-avatar-vrm`    | Early: turn a rigged character's rest pose into the VRM T-pose facing +Z on a copy, and judge the before and after pose sheets with you.                                                                                                      |

The rig skill carries headless Blender scripts (a humanoid set and a creature set with an example config), and the avatar skill the rest pose script Maquette itself runs. Everything else is instructions.

## What to say, and what the app asks of you

Each skill triggers from a plain request. Here is one prompt per skill and what you are asked to approve or decide in the app.

### `maquette-lab-round`

Prompt: "Start a concept round for my site project: three directions for the lantern post, then show me the options."

You pick the concepts in the Lab; the agent only presents them and waits. If the round runs on OpenAI Images instead of Codex, an Approve prompt shows the estimated cost before anything is sent. Deny cancels its jobs. The monthly cap stays yours, in Settings.

### `maquette-by-hand-round`

Prompt: "Write By hand handoffs for the five waiting props, then match what I download and show me the inbox checks."

By hand handoffs write folders and spend nothing. A send that goes to Tripo or Meshy through the API waits for you to Approve the credits in the app, and the agent never sends it again while it waits. Matching a download is done by looking at its picture next to the waiting sends, and you confirm any look-alike in the New downloads dialog. Accepting a model into the project is a tool call the agent makes after it reads the checks and tells you what failed.

### `maquette-character-rig`

Prompt: "The guardian from the inbox has hair weighted to a finger bone. Run the rig check and fix the skin weights."

Nothing here costs credits or ships. The agent works on a copy, runs a pose test, fixes weights per island and adds the result as the next version of the asset, so v1 stays. Moving a joint changes the rest pose, so the agent asks you first and shows the evidence.

### `maquette-game-export`

Prompt: "Import the models from my game repo, set a 5k triangle budget for props, run the pipeline and tell me what changed in git."

Ship always waits. The run stops at Approve and ship in the app, with the counts of the plan, and you press Approve or Deny. The agent polls the run, never starts it again and never commits in the game repo; you review the changed files in git.

### `maquette-avatar-vrm`

Prompt: "Normalise the duelist's rest pose for VRM and show me the pose sheets."

A new rest pose changes how every clip plays, so nothing uses the normalised model until you say yes to the before and after sheets. The agent works on a copy and never changes skin weights on its own: a cape or hair that follows the arms is shown to you, with a proposed fix.

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

To update the skills later, run `/plugin marketplace update maquette`, then update the plugin from the `/plugin` menu (the plugin version in `plugin.json` changes with each release, which is what tells Claude Code there is something new). The skills then appear as `/maquette:maquette-lab-round` and so on, and Claude picks them up from what you ask for.

The plugin also declares the Maquette MCP server, so the tools come with it. Its `.mcp.json` reads the token and the port from your environment (`${MAQUETTE_MCP_TOKEN}` and `${MAQUETTE_MCP_PORT:-4791}`), so no token is written into a file. Set them in the shell you start Claude Code from. Copy the token from Settings > Agents:

```sh
export MAQUETTE_MCP_TOKEN="<token>"
export MAQUETTE_MCP_PORT=4791   # only when you changed the port
```

On Windows, in PowerShell, `$env:MAQUETTE_MCP_TOKEN = "<token>"` sets it for that window only; `setx MAQUETTE_MCP_TOKEN "<token>"` keeps it for windows opened afterwards (open a new one). On a Mac, a Claude Code started from the Dock or a launcher does not see variables from your shell profile: start `claude` from a terminal where you ran the export.

Maquette must be open with the server on before the tools can answer (see [Connect an agent](/docs/agents/connect/#before-you-connect)). If you started Claude Code first, run `/mcp` and reconnect Maquette.

Use this when you install the plugin. The `claude mcp add ... --header "Authorization: Bearer <token>"` line from [Connect an agent](/docs/agents/connect/) is the way to add the server without the plugin, and it stores the token as it is in Claude Code's own config. If you already added the server that way, keep one of the two entries so the tools are not listed twice.

## Codex

Codex finds skills in `$HOME/.agents/skills` and in a repository's `.agents/skills`, and follows symlinked folders. Clone the repo once and link the skills, so one source serves both agents and `git pull` updates them:

```sh
git clone https://github.com/Shironex/maquette.git
node maquette/plugins/maquette/install-codex.mjs
```

That links each skill into `~/.agents/skills`. Use `--repo <path>` to link into one repository instead, `--copy` to copy rather than link, and `--remove` to take them out again. On Windows the link is a directory junction, which needs no administrator rights. Restart Codex if a skill does not show up, then type `$` to mention one.

Add the server to `~/.codex/config.toml`. Here too the token stays in the environment: Codex reads it from the variable named by `bearer_token_env_var`, so export `MAQUETTE_MCP_TOKEN` as above. The `http_headers` form in [Connect an agent](/docs/agents/connect/) writes the token into the file instead.

```toml
[mcp_servers.maquette]
url = "http://127.0.0.1:4791/mcp"
bearer_token_env_var = "MAQUETTE_MCP_TOKEN"
tool_timeout_sec = 150
```

## Keeping the skills honest

`pnpm lint:meta` (part of `pnpm gate`) runs `scripts/check-skills.mjs`. It reads the tool list the server's tool router writes to `apps/desktop/src-tauri/tool-names.txt` (the same list the MCP handshake test checks) and fails when a skill names a tool the server does not have, when a skill's name does not match its folder, when its description does not say when to use it, when a skill passes 250 lines, when a dash slips in, or when the plugin's MCP entry carries a literal token.

Next: [MCP tools](/docs/agents/mcp-tools/).
