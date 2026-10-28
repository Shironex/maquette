---
title: Parts
description: Split a model that came as one mesh into named parts, so object states, explode tracks and material tweaks can move and change each one.
sidebar:
  label: Parts
---

Tripo and Meshy send a model as one merged mesh. Object states, Timeline explode tracks and per material tweaks need named parts: a lantern's lid that lifts, a chest's lid that opens. Parts turns one mesh into named parts, free and on your machine.

## Split into parts

A prop that is one mesh with several loose pieces gets the Inbox check **One mesh, no parts**. Its button adds the **Split into parts** fix-up, which runs headless in Blender on a copy. It splits one of two ways:

- **By loose pieces** (the default): each piece of geometry that touches no other becomes a part.
- **By material**: the faces that share a material become one part. A lid and a post that are both stone end up as one part.

**Merge bits under** (1% by default, up to 20%) joins every piece smaller than that share of the model into the nearest part, so stray bits on a cage do not become parts of their own. Preview the numbers runs it and shows the parts, draw calls and triangles before and after, and what the other way would give: "By loose pieces: 5 parts, 9 pieces, 4 tiny bits merged in. By material: 4 parts." The split keeps the maps and UVs, so the texture stays. The Inbox preview's **Parts** mode shows the split copy with each part in its own colour.

New parts are named `part_1`, `part_2` and on, top first, each with its origin at its own middle so it turns around itself. Like every fix-up, the split never changes the model file; [Build scene](/docs/pipeline/build-scene/) applies it on every version.

## The Parts tab

The Inspector's **Parts** tab shows one model's parts as it ships them. Pick the model at the top of the side panel.

- **Select**: click a part in the view or the list; Shift adds to the selection.
- **Rename**: double click a part in the list. The name is what the export calls the part's node, and what object states and explode tracks pick it by, so name parts before you use them. Names are unique and at most 64 characters.
- **Merge**: joins the selected parts into one, named after the first. The join ships in the next build; the list's split button undoes it.
- **Hide**: draws the part as a ghost here, to see what is behind it. A hidden part still ships.

The view colours each part, or shows it shaded or as wireframe. The side panel lists the selected parts' materials and triangles. Under the view, the export card lists the node names and the Motion states card the states and explode tracks that pick parts by name. A model that is still one mesh offers **Split into parts** right there.

Names, joins and hidden parts are kept per asset in `prepare.json`, under `parts`, next to the fix-ups.

## Before and after

**Before and after** in the Parts tab puts the model as it came next to the split, in one canvas with one camera, as Compare does. It colours each part, shades or draws wireframe. When there is no split, it shows the version before against the newest. Under it a table compares the parts Motion can move, draw calls, triangles, textures, transfer size and Inbox checks; a dot marks the better number.

**Keep** picks a side. Keeping the model as it came takes the split fix-up off; keeping an older version makes it the one the Board and the next export use. Every version stays in the asset either way. In a [game project](/docs/start/game-projects/), which has no Timeline, **Explode parts** pulls the parts apart from the middle on both sides, to see where each one ends.

## For agents

`add_fixups` with `op: "split-parts"` adds the split, `get_parts` lists the parts, `set_parts` renames, joins and hides them, and `compare_versions` returns the numbers of two sides. See [MCP tools](/docs/agents/mcp-tools/).
