---
title: Plan
description: Plan on canvases of notes, pictures, links and your assets, beside agents, and plan a website's camera tour.
sidebar:
  label: Plan
---

Plan holds a project's canvases. Each canvas is one file in `plans/` in the project folder, so it syncs and versions like the rest of the project. A canvas holds notes, pictures, web pages, assets of the Board and Lab directions, joined by links. Agents can add to a canvas over MCP while you work in it, and the Changes panel shows every change, yours and theirs, to undo.

## The list and the rail

The list shows the canvases edited last as cards, then all of them in a table with Rename and Remove. A long name takes two lines on its card; hover it to read all of it. With no canvas yet, the list offers the presets (Brainstorm, Story, Game mechanics) and a prompt to copy for an agent that starts one.

Under Plan on the sidebar, the three canvases you opened or edited last are one click away. A dot after one says it changed since you last had it open, by an agent or a sync from another computer. The list follows the open project, renames and removals, and the times stay on this computer.

## A canvas

The bar on top adds a note (`N`), a picture (`I`), a web page (`L`) or assets of the Board and Lab directions (`A`). Double click anywhere to write a note there. An empty canvas lists each way to start with its button, and a prompt to copy for an agent.

An asset or direction node keeps only its id and reads the rest live from the project. One that is no longer in the project says so, with what to do: restore it from Removed on the Board, or delete the node.

On a narrow window the map folds into a Map button under the zoom, so it covers no nodes; press it to show the map again. Plan works down to a 1120 by 720 window.

## The camera plan

A website project has one built in canvas, the camera plan, pinned in the list. Its nodes are the camera stops of `layout.json` in tour order, and nothing reaches the site until you press Apply. With no stops yet, Add stop in its panel starts the tour. A stop has no render until the pipeline's [Capture posters](/docs/pipeline/capture-posters/) step makes one.

When `layout.json` changes underneath, the panel offers Compare, Rebase and Start over. If it changes again while the comparison is open, the rebase is refused and the comparison asks you to review it again, so nothing is taken in that you did not see.
