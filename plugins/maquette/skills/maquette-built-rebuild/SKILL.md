---
name: maquette-built-rebuild
description: Rebuild a generated model (a Tripo or Meshy scan, a library model) as a built asset, a Blender script in the project, and put it in the old model's place. Use when asked to rebuild, remake or clean up a prop as a built asset or a script, when a scanned prop looks scratched or blobby up close, or for "rebuild the desk lamp as a built asset". Not for new concepts (maquette-lab-round) or provider sends (maquette-by-hand-round).
---

# Maquette built rebuild

A built asset is a model a Blender script in the project makes: no credits, no approval, clean up
close. This skill turns a generated model into one, through at least three render, critique and
fix rounds against the ref, then swaps it in where the old model stood, and stops before Ship so a
person looks first. Tools are Maquette MCP tools (`/docs/agents/mcp-tools/`); the helpers are the
guide at `/docs/guides/built-assets/`.

## Before you start

Call `get_started` first. It says whether a workspace is open, which tools work, which keys are saved
(true or false, never a value), every project with its stage, the gates only a person can pass, and
a `next` list; it is short, and `detail: true` gives every row and gate. No project to work in: `create_project` (`target` is `website` or `game`) makes one and spends nothing. No tool removes a project; a removed project's id fails with not-found, and only a person restores it. If the Maquette
tools fail with `ECONNREFUSED`, the app is closed or its server is off: ask the person to open
Maquette and turn the server on in Settings > Agents. An error names its `code`, and `fix` and
`tryTool` when they help; `personOnly` means ask the person instead of trying again.
Settings > Agents can turn tool groups off, all but start and project: their tools are missing from
your list (or answer code `group-off`), and `get_started` names them in `toolGroupsOff`. Any tool this
skill names may be off. When a step needs a missing tool, do not work around it: ask the person to
turn its group on in Settings > Agents, then go on.
A project's planning canvases (the plan group) hold the person's notes, links and plans: `list_canvases`
lists them and `get_canvas` reads one, so read the canvas the person points at before you plan for them.
Agents never delete a person's nodes: ask with `propose_removal`, and only the person answers.
A website project's camera plan (`get_camera_plan`) is tried there until the person presses Apply
in the app: agents may propose a tour, never apply one.

## Hard rules

- Never run Ship. Stop after the swap with the side by side renders; `run_pipeline` with
  `buildOnly: true` is the furthest you go, and only when the person asked for a build.
- Visual work gets its time: no first pass handed back. At least three rounds (step 4), each with a
  written fault list, and every round's renders kept.
- Never capture the screen. Every picture is a file `render_views` writes headless; read the PNGs.
- Work in a scratch folder outside the repo and the workspace for notes and fault lists. The
  script itself lives in the project's `built/` folder once added.
- Only undo a replace you made yourself in this session (`list_built_assets` says when an asset
  was replaced, not who did it), and never delete a version or a card yourself.

## Time budget

Set it before round one and say it: 60 to 120 minutes for one prop, about 15 per round (a build is
seconds, a `render_views` set about a minute, the critique is the real work). A prop with many
parts (a lamp with a hinged arm, a stack of books with a mug) takes the top of the range. When the
budget runs out, stop and report the open faults with the last renders; do not swap in a model
whose fault list still has a fault in the first four rows of the quality bar.

## 1. Read the asset to replace

The person names it ("the desk lamp"). `list_assets` `{ project }` finds its id.

1. `get_model_facts` `{ project, asset }`: `atRealSize` (the box the layout places, in metres; your
   target size), `facing` (its front axis and whether it was measured or assumed), `counts`
   (triangles and materials: the budget below starts here), `layout` (where it stands) and `refs`
   (the ref images, whole paths). `axes` says how to read it: glTF y up, front +z, which Blender
   imports as -y. A facing that says assumed is checked in the round 0 `top` view below.
2. `get_ref_views` `{ project, asset }`: which ref shows which side. The front ref leads the
   critique. No ref at all: ask the person for one, or make one with `maquette-lab-round`.
3. `get_material_tweaks` `{ project, asset }` and `get_parts` `{ project, asset }`: the old model's
   material names with their stored tweaks, and its named objects. Neither gives base colours:
   read colours from the renders and the ref. A page may look up an object by name (a lid, a
   switch): keep those names.
4. `get_style_board` `{ project }`: the project's palette (#rrggbb).
5. `list_built_assets` `{ project }`: the project's other built assets and their scripts, the style
   to match. If the asset to replace is listed there already, it is built: skip the swap, edit its
   script and use `rebuild_built_asset` (step 6).
6. Round 0: `render_views` `{ project, asset }` on the old model, with its own fix-ups and real
   size applied. Its `sheet` (refs first, then the views) and its close ups are what the rebuild
   must beat; its `top` view is the facing reference for step 5.3. Write down what is wrong with it.

## 2. Rules for the script

- Real size, in metres, the old model's orientation: Blender space is z up, the front faces -y,
  the origin at the bottom centre (`mq.ground`). Match `atRealSize` to 2 percent per axis.
- Palette colours first: take each colour from the style board, then from the old model's
  renders and the ref; a colour of your own only when none fits, and say which. One
  `mq.flat_material` per colour, named for what it is (`cover`, `brass`, `shade`).
- A triangle budget: a game project's budget for its folder (`get_game`), else the number the
  person gave, else at or under the old model's count. Spend triangles on silhouettes and rims, not
  on flat faces.
- No image textures and no UV maps: flat colours only, as the helpers make them.
- A part that may animate (a lid, a switch, an arm, a drawer) is its own child object with its
  origin at its pivot (`mq.set_origin`), parented with `mq.parent_to`. Everything else may be
  joined (`mq.join` keeps every face's material).
- No coplanar faces closer than half a millimetre: a label, a band or a page edge sits at least
  0.5 mm proud of the face under it, or is cut in with `mq.boolean`. Closer faces flicker on a page.
- Use the helpers: `import maquette_helpers as mq`, and list `built/maquette_helpers.py` in
  `helpers`. `built/maquette_example.py` shows each helper once. `mq.lathe` and `mq.fillet` for
  turned parts, `mq.extrude` with `mq.rounded_rect` for slabs and covers, `mq.tube` for cables and
  handles, `mq.page_block` for pages, `mq.crisp_edges` instead of bevels on curved edges.
- The script's file name is its module name: lowercase letters, digits and underscores
  (`desk_lamp.py`). Its `build()` returns the root object.

## 3. Add it

`add_built_asset` `{ project, script, helpers: ["built/maquette_helpers.py"], name, kind: "prop" }`
copies the script into `built/` and builds it at once. Give a `name` other than the old asset's
(for example "Desk lamp built") so the two never clash. Read the answer's `build`: `result`
(built or failed), `version`, `objects`, `materials`, `triangles` and `warnings`. A failed build
carries `error` and the end of Blender's log: fix the script and rebuild.

From here on edit the copy in the project's `built/` folder (beside the `refs/` folder that the
ref paths from step 1 sit in), then `rebuild_built_asset` `{ project, asset }`. Each good build is the
asset's next version, so each round has a version of its own.

## 4. Render, critique, fix: at least three rounds

Each round:

1. `render_views` `{ project, asset: <built>, compare: <previous version> }`. From round two on,
   `compare` renders the round before with the same cameras. Use `closeUps` (`{ node }` or
   `{ box }`) to frame the part a fault sits on, and `angle` when the ref was drawn from elsewhere.
   An answer with status `rendering`: call again with the same arguments.
2. Read `sheet` and every close up. Hold the three-quarter view beside the front ref.
3. Write the fault list for the round to `round-<n>.md` in your scratch folder: each fault, the
   picture it is seen in, and the element of the quality bar it breaks. Then the fixes you made.
   An empty list in round one or two means you did not look closely: look at the close ups again.
4. Fix the script and rebuild. A rebuild that adds no version changed nothing in the model.

Renders stay in `exports/renders/<asset>/views/v<n>/` per version (a compare in `v<n>-vs-v<m>/`,
a set with `closeUps` or `angle` in a folder of its own ending in `-<8 hex>`), so every round's
pictures are kept without copying. Name them in each round's fault list.

Stop the loop only when a round's list has no fault in the quality bar's rows, and never before
round three. A fifth or sixth round is normal for a prop with several materials.

## Quality bar per element

| Element             | Passes when                                                                  |
| ------------------- | ---------------------------------------------------------------------------- |
| Colour              | Each material reads as its palette colour in the page light: no pale covers. |
| Materials           | Every material of the old model and the ref is in `materials`, none lost.    |
| Curves and rings    | No facets at the close up: rims, rings and handles read round.               |
| Edges and seams     | No hairlines on spines or joins, no flicker from coplanar faces.             |
| Silhouette          | The three-quarter view matches the ref's outline and proportions.            |
| Size and facing     | Within 2 percent of `atRealSize`, front where the old front was (step 5).    |
| Parts               | Each part that may animate is a named child with its pivot at the joint.     |
| Budget and warnings | Triangles under the budget; every build warning read and explained.          |

The light is the page's own (the `light` field of `render_views`): a colour that reads right
there reads right on the page, so judge colour only in these pictures.

## 5. Proof

1. The export, not the scene: `render_views` loads the built model file (`models/<id>_v<n>.glb`,
   with the id's hyphens as underscores) into a fresh Blender, so its pictures are the re-import of
   the export. Check its `boundsRendered` against `atRealSize`, and `get_model_facts` on the built
   asset for its counts. Its `facing` is always assumed by convention for a built asset, so it
   proves nothing: the top views below do.
2. A dry run first, `replace_with_built_asset` with `dryRun: true` and the arguments of step 6:
   it lists every setting as kept, carried, reset or brought, what no longer resolves (a part or
   clip the new model lacks) and the old and new min and max. Each unresolved name is a fault.
3. Facing, top down, before the swap: hold the built asset's `top` view beside the old model's
   round 0 `top` view. Both look straight down with the front at the bottom of the picture, and
   the old one carries its own fix-ups, so the same side must face down in both. A model turned
   the wrong way: turn it in the script and rebuild, then look again.
4. The side by side, after the swap in step 6: `render_views` `{ project, asset: <old>, compare:
<the version before> }` renders old and new with the same cameras. It renders both with the
   asset's settings as they are after the swap, which reset the fix-ups to the built model's: the
   old row shows the old file without its own fix-ups. So it proves facing only when the old
   model had none (`list_fixups` before the swap); otherwise step 3 decides, and a turned old row
   is said in the report, not fixed.

## 6. Finish

1. Swap: `replace_with_built_asset` `{ project, asset: <old>, built: <built> }`. The old asset keeps
   its id, layout, camera stops and ship file name, and gains the script; the real size carries,
   fix-ups and material tweaks reset to the built model's. The built card goes to the asset trash.
   It is refused for a built asset that is not current: rebuild first. An asset that was already
   built is never swapped: `rebuild_built_asset` on it is its new version.
2. Render the side by side (step 5.4) and keep its `sheet` path, with the round 0 and last round
   `top` views of step 5.3.
3. Stop. Do not call `run_pipeline` without `buildOnly`. The person looks at the side by side and
   at the Board, and ships when they are happy. `replace_with_built_asset` with `undo: true` brings
   the old model back as it was, while no version came from elsewhere.

## Stop conditions

- Stop and ask when the asset has no ref, when two refs disagree on a feature, or when the
  budget runs out with faults open.
- Stop when the dry run lists names a page needs that the new model lacks; ask before swapping.
- Finish with: the side by side `sheet` path, the round count and each round's fault list path,
  the versions, the triangles against the budget, any colour you chose yourself, and that Ship
  waits for the person.
