---
title: Use these docs with an AI agent
description: I write every page so an agent can read it as well as you can. Here is what is published for agents and how to point one at it.
sidebar:
  label: Use these docs with an AI agent
---

## What is published

| File                  | What it holds                          | Use it to                                     |
| --------------------- | -------------------------------------- | --------------------------------------------- |
| `/llms.txt`           | One line per page, with its `.md` link | Let the agent pick the pages it needs.        |
| `/llms-full.txt`      | Every page in one Markdown file        | Give a long context agent everything at once. |
| `/docs/<page>.md`     | One page as Markdown                   | Paste or fetch one topic.                     |
| Copy page as Markdown | The same `.md`, on your clipboard      | Paste into a chat.                            |

Every page has a Copy page as Markdown button and a View as .md link under its title.

## /llms.txt

An index in the [llms.txt](https://llmstxt.org/) format: a title, one line on what Maquette is, then one link per page with one line about it, grouped like the sidebar.

```markdown title="/llms.txt"
# Maquette

> Maquette turns AI concept art into real-time 3D web scenes. Docs for Maquette 0.1.0.

## Start

- [Install](https://shironex.github.io/maquette/docs/start/install.md): Get Maquette running...
```

## Point an agent at it

1. Copy the address of `/llms.txt`, or of one page's View as .md link.
2. Paste it into Claude Code or Codex with your question.
3. For work in the app itself, also [connect the agent over MCP](/docs/agents/connect/). The docs explain. The tools act.

```text title="prompt"
Read the llms.txt of the Maquette docs, then the pages you need.
Tell me which MCP calls you would make to set up Compress for my project
before you make them.
```

## What stays stable

- **Heading anchors.** Each heading keeps its id, such as `#settings`.
- **Version line.** Each page says which app version it applies to and when it last changed.
- **Plain Markdown.** The `.md` twin has no scripts or widgets. Callouts become quoted lines that start with Note, Caution or Costs credits.

:::note[Docs or MCP?]
The docs tell an agent what Maquette does. The MCP server lets it do it.
:::
