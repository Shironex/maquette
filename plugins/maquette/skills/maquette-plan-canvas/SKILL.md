---
name: maquette-plan-canvas
description: Plan beside a person on a Maquette planning canvas: read the canvas and its change log, add notes, links, pictures, assets and Lab directions near the right node, edit or tidy them, and undo your own run. Use when asked to brainstorm, plan mechanics or a story, research and pin findings, clean up what an agent added on a canvas in the Plan view, or propose a camera tour for a website project's camera plan. Not for making concepts (maquette-lab-round).
---

# Maquette planning canvas

A planning canvas is a board of nodes (note, image, link, asset, direction) and links between them,
one file per canvas in the project's `plans/` folder, open in the app's Plan view. A person and
agents write to it at the same time: every write lands in the canvas's change log, the person sees
an agent's nodes with its name on them, and they can undo any agent change from the Changes panel.
Tools are Maquette MCP tools (`/docs/agents/mcp-tools/`); `project` is the id `list_projects`
returns and `canvas` the id `list_canvases` returns.

## Before you start

Call `get_started` first. It says whether a workspace is open, which tools work, which keys are saved
(true or false, never a value), every project with its stage, the gates only a person can pass, and
a `next` list; it is short, and `detail: true` gives every row and gate. No project to work in: `create_project` (`target` is `website` or `game`) makes one and spends nothing. No tool removes a project; a removed project's id fails with not-found, and only a person restores it. If the Maquette
tools fail with `ECONNREFUSED`, the app is closed or its server is off: ask the person to open
Maquette and turn the server on in Settings > Agents. An error names its `code`, and `fix` and
`tryTool` when they help; `personOnly` means ask the person instead of trying again.
A project's planning canvases hold the person's notes, links and plans: `list_canvases` lists them
and `get_canvas` reads one, so read the canvas the person points at before you plan for them.
Agents never delete a person's nodes: ask with `propose_removal`, and only the person answers.
A website project's camera plan (`get_camera_plan`) is tried there until the person presses Apply
in the app: agents may propose a tour, never apply one.

## Hard rules

- Never remove a person's node or link, or a node of yours a person wrote in (`get_canvas`
  shows `personWrote: true`; it is theirs now). `remove_nodes` refuses it; ask the person with
  `propose_removal` instead. Only the person answers a proposal; no tool accepts one, yours included.
- Marking a mechanics overlap as fine and the Auto layout after agents switch are the person's.
- Pass the same `agent` name on every call, such as "Research agent", and a `run` id that groups
  one task's calls, so the person can read and undo the whole run at once.
- Agent names are labels, not accounts. Leave nodes made under another agent's name alone unless
  the person asks; nothing but courtesy stops you, so do not count on it either way.
- Edit a person's content only when they asked for it. The change log holds their earlier version
  until your change is released: the person presses Undo or Keep (Keep run for a whole run) in the
  Changes panel, or you `undo_run` it. That room is limited (4 MB): when it is full, edits to a
  person's content are refused until something is released. Adding your own nodes still works, and
  `undo_run` is never refused for lack of room.
- Keep each call small: a few nodes at a time. A call whose undo record would pass 256 KB is
  refused with nothing written; split it.
- Never capture the screen. The person looks at the canvas in the app.

## 1. Read the canvas

1. `list_canvases` `{ project }`: ids, names, node and link counts, when each last changed. No
   canvas yet and the person wants one: `create_canvas` `{ project, name, preset }`. The preset
   only sets the start: `brainstorm` (blank, the default), `story` (beats left to right with
   characters above and places below, worded for a game or a website) or `mechanics` (a card per
   system with its verb, look and input). The starter notes are guidance under your name and one
   change in the log with no run (the answer's `change` id is what `undo_run` takes); fill them in
   with `update_nodes` or leave them for the person to replace. A blank canvas has nothing to undo,
   so its answer has no `change`.
2. `get_canvas` `{ project, canvas }`: nodes with id, kind, title, text cut at 120 characters,
   x and y, and `by` (absent for a person's node), and the links. Pass `full: true` or `nodes`
   for the whole text of a few nodes. It also answers `autoLayout`, `proposals` (your asks still
   waiting, `stale` when a node changed or went) and, on a Game mechanics canvas, `overlaps`: two
   system cards whose verb, look or input lines share a main word. Point out an overlap that is not
   `fine` and suggest a different verb, look or key; the person decides, and may mark it as fine.
3. `get_changes` `{ project, canvas }`: the change log, newest first. Read it when you come back
   to a canvas, to see what the person changed since your last run (`by: "person"` filters).

## 2. Add and edit

- `add_nodes` `{ project, canvas, agent, run, near, nodes, links }`: leave out x and y and the
  nodes are laid out in a column beside `near`, clear of other nodes. Link to this call's nodes
  with `$0`, `$1`. Everything is checked before anything is written. Answers with the new ids and
  the change id.
- `update_nodes`: only the fields given change (title, text, url, x, y, width). If the person
  edits the same field before their canvas saves, their value wins.
- `layout_canvas`: moves your overlapping nodes to a free spot beside the node they link to. A
  person's nodes, yours a person wrote in and yours a person dragged (`personPlaced`) never move, and naming one is refused; give an explicit x and y to move it. The x and y you give are kept. While
  Auto layout after agents is on (the default), `update_nodes` moves your edited nodes you gave no
  x or y clear in the same change (`autoLayout.moved` in the answer); when the person turns it
  off, `layout_canvas` is refused, so place nodes with x and y.
- `remove_nodes`: your own nodes and links only. A node goes with its links.
- `propose_removal` `{ project, canvas, agent, run, nodes, reason }`: asks the person to remove
  nodes you may not remove. The canvas does not change; they answer Keep it or Remove in the
  Changes panel. Give a short reason. A proposal for a node that changes or goes is stale.
- `start_from_node` `{ project, canvas, node, name, agent }`: a new canvas from a node and the nodes
  linked to it, as one change you or the person can undo. The source canvas never changes.

Tell the person in one line what you added and where (near which node), and the change id or run.

## 3. Undo

`undo_run` `{ project, canvas, agent, run }` (or `changes`, at most 100 ids from `get_changes`)
undoes your own changes carefully: a field someone changed since stays as they left it, and a node
a person edited or linked since is kept with their links (`kept` counts them). It answers with how
many changes it undid; `nothingChanged: true` means everything was changed since, and the changes
still count as undone. A change a later agent change rewrote on top of waits: undo the later one
first, or name both (or their run) to undo them together. An undo can not be undone; make the
change again instead. A change the person kept is theirs: `undo_run` refuses it, so ask them. The person can undo or keep any agent
change from the Changes panel.

## 4. Camera plan (website projects)

A website project's camera plan is canvas `camera-plan`: stop nodes in tour order, the person's main
line on top. Nothing on it reaches the site until the person presses Apply, and only the person
applies: no tool writes `layout.json` from a plan, and `undo_run` refuses an Apply.

1. `get_camera_plan` `{ project }`: the main line with each stop's settings, any proposals and
   `stale: true` when `layout.json` changed since the plan was based on it (tell the person; they
   compare or rebase in the app, and a rebase is refused when `layout.json` changed again after the
   comparison they saw). A plan with no stops means `layout.json` has none yet: propose a tour, or
   tell the person to use Add stop in the camera panel.
2. `propose_camera_plan` `{ project, agent, run, stops }`: the whole tour in order as your own line
   below theirs. Known stops by id take only the settings you give; a new stop needs a name,
   position and target. A new proposal replaces your last one.
3. `update_nodes` with `stop: { name?, position?, target?, fov? }` changes one stop node's settings,
   only when the person asked for it on their main line.
4. `get_apply_preview` `{ project }`: what Apply would write, every moved, added, removed and
   changed stop. Tell the person in one line what changes and that they apply it in the app.
