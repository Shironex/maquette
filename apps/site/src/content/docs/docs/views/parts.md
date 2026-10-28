---
title: Parts
description: Split a model that came as one mesh into named parts, free in Blender or on Tripo or Meshy, and assemble one prop from part models, so object states, explode tracks and material tweaks can move and change each one.
sidebar:
  label: Parts
---

Tripo and Meshy send a model as one merged mesh. Object states, Timeline explode tracks and per material tweaks need named parts: a lantern's lid that lifts, a chest's lid that opens. Parts turns one mesh into named parts, free and on your machine, or with Tripo's or Meshy's own split when the pieces are welded into one shell. It also joins parts made as separate models into one prop.

## Split into parts

A prop that is one mesh with several loose pieces gets the Inbox check **One mesh, no parts**. Its button adds the **Split into parts** fix-up, which runs headless in Blender on a copy. It splits one of two ways:

- **By loose pieces** (the default): each piece of geometry that touches no other becomes a part.
- **By material**: the faces that share a material become one part. A lid and a post that are both stone end up as one part.

**Merge bits under** (1% by default, up to 20%) joins every piece smaller than that share of the model into the nearest part, so stray bits on a cage do not become parts of their own. Preview the numbers runs it and shows the parts, draw calls and triangles before and after, and what the other way would give: "By loose pieces: 5 parts, 9 pieces, 4 tiny bits merged in. By material: 4 parts." The split keeps the maps and UVs, so the texture stays. The Inbox preview's **Parts** mode shows the split copy with each part in its own colour.

New parts are named `part_1`, `part_2` and on, top first, each with its origin at its own middle so it turns around itself. Like every fix-up, the split never changes the model file; [Build scene](/docs/pipeline/build-scene/) applies it on every version.

## Split on Tripo or Meshy

**Split into parts...** in the Parts tab opens the split sheet with three ways: **In Blender, free** (the fix-up above), **Tripo** and **Meshy**. The Inbox's One mesh, no parts check opens it too, on a model a send brought, before it is accepted. The sheet shows the key from the keychain, an estimate from the provider's published price list, and asks once more before any credit goes out ("Split Frost lantern with Tripo? 1 segment task, semantic model, balanced. About 40 credits. Tripo cannot cancel once it is sent.").

- **Tripo** segments the mesh. **Geometry** cuts where the shape changes; **Semantic, beta** looks for parts that mean something, like a lid or a cage, with a granularity (Simple, Balanced, Detailed) and loose pieces split too by default. **Complete hidden parts** runs a second Tripo task that rebuilds faces one part hid in another, for more credits. A model Tripo made goes by its task; any other goes up as a file. The parts arrive as `tripo_part_0`, `tripo_part_1` and on; name them in the Parts tab.
- **Meshy** runs Auto Split on a model Meshy made with Meshy 6 or 7 (Low Poly and Smart Topology models cannot be split). **Auto** lets Meshy choose the cuts; **By parts** and **By colour** take 1 to 10 names to look for ("lid, cage, crystal"). About 10 credits, refunded when the split fails, and you can cancel while Meshy has not started.

A split is a send like any other: it waits in the 3D queue, its credits show in the Credits sheet, it carries on after a restart, and an agent's split waits for your approval in the app. It lands in the Inbox; accept it and it becomes the asset's next version. The version it came from stays, so Before and after compares the two and **Keep** picks one.

## The texture after a split

Meshy's split never keeps the texture: the parts come back grey. Once you accept them, Maquette bakes the texture back from the version they came from onto the parts, headless in Blender and free, and adds the result as the version after. Tripo does not say whether a split keeps the texture, so the Inbox looks when it arrives; when it is gone, the Parts tab offers **Bake back** instead of running it. Either way the grey version stays: **Keep** it from the note in the Parts tab to undo the bake. A bake that fails says why and can run again.

## Assemble one prop

Parts made as models of their own (from [object refs](/docs/views/lab/), say, one model per part) join into one prop with **Assemble a prop...** in the Parts tab. Pick the parts and their order, give each the node name it ships as and a pivot (bottom centre, centre, left or right hinge, or its own origin), and place them where their models put them or by numbers, in metres. Headless Blender joins them into one GLB with a node per part under one root named after the prop, and it lands as the next version of a new prop, or of the prop you pick. Each part stays an asset with its own versions; the recipe is kept in `assemblies.json` in the project, so assembling again picks up a part's newer version. A door on its left hinge swings on its hinge in object states.

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

`add_fixups` with `op: "split-parts"` adds the free split, `split_with_provider` splits on Tripo or Meshy (it waits for your approval), `get_parts` lists the parts, `set_parts` renames, joins and hides them, `compare_versions` returns the numbers of two sides and `assemble_prop` joins part models into one prop. See [MCP tools](/docs/agents/mcp-tools/).
