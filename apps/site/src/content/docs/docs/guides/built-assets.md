---
title: Built assets
description: Make a model with a Blender script in the project. Maquette ships the helpers a prop script needs (shapes, page lines, a lathe, crisp edges, brass and glazed ceramic) and an example that uses every one.
sidebar:
  label: Built assets
---

A built asset is a model a Blender script in the project makes. It costs no credits and waits for no approval. The script is the source: change it and the model builds again as the asset's next version.

Use one for a prop that should be clean up close: a book, a mug, a lamp, a laptop with a lid that opens. A scan from a provider is quicker for shapes nobody would write by hand.

## Add one

**Built asset** in the Board's Add menu (or `add_built_asset` over MCP) takes the script, the helper files it imports or reads, a name, a kind and, optionally, a root object.

- The script and its helpers are copied flat into the project's `built/` folder. From then on they are the project's: edit them there.
- A helper several assets share is kept once. A different file under a name that is already there is refused unless you choose Replace, which keeps the file it writes over as `<name>.bak` beside it.
- The asset lands at Blender on the Board and builds at once, headless. Blender's window never opens.

## What the script does

```python
import maquette_helpers as mq


def build():
    glaze = mq.glazed_ceramic("glaze", "#e9e4d8")
    profile = [(0, 0), (0.041, 0), (0.041, 0.095), (0.0355, 0.095), (0.0355, 0.007), (0, 0.007)]
    return mq.lathe("mug", mq.fillet(profile, (0, 0.012, 0.004, 0.004, 0.008, 0)), glaze)
```

- The script is imported as a module named after its file. When it has a `build()` function, that is called, and the object it returns is exported with everything parented under it. Otherwise the root object option names the object, and without either the whole scene is exported.
- Blender space: z up, in metres. The export turns it into glTF's y up.
- Object names, parents, origins, custom properties and materials reach the model as the script made them. So a page can look up `lid` or `bell_button`, and a hinge keeps its pivot. Cameras and lights are not exported.
- It runs in a scratch copy under the workspace cache, one build at a time, with a 10 minute limit. A script that writes next to itself never writes into the project.

A good build becomes `models/<id>_v<n>.glb`. The Inbox checks still run on it and the ones that do not pass stay on the build as warnings. The real size, fix-ups, layout and material tweaks apply to a built model as to any other.

## When it builds again

`built.json` in the project records each built asset's script, helpers and last build. The card says **Script changed** when the script or one of its helpers differs from what the last good build used, and **Build failed** with the script's error when it did not build. Rebuild on the card always runs Blender. A full pipeline run first builds the new and changed built assets, and does not start while one is missing a file or fails.

## Helpers

Maquette ships a helper file for these scripts: `built/maquette_helpers.py`. A new project has it from the start. An older project gets it from the Update banner in the [Pipeline view](/docs/views/pipeline/#starter-scripts), or from `update_starters`.

To use it, a script imports it and its asset lists the file among its helpers:

- in the script: `import maquette_helpers as mq`
- in the Add built asset dialog: **Add files...** under Helpers, then `built/maquette_helpers.py` in the project folder
- over MCP: `helpers: ["built/maquette_helpers.py"]` in `add_built_asset`

The file is then staged with the script on every build and counts in what the build is made from.

How the helpers work together:

- A shape helper returns its object, already in the scene. `location` is the bottom centre of the shape (a sphere's centre), so a piece stands where it is put.
- Every shape comes shaded smooth with its hard edges kept crisp. None needs a bevel to look clean.
- A material is one flat colour: no image and no UV map. A colour is `"#rrggbb"` as a colour picker shows it.
- `join` and `finish` never drop a material. Every face keeps the one it had, whatever order the pieces come in.

Each helper has a docstring in the file with its arguments. One line each:

### Materials

| Helper           | What it gives                                                                                                           |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `flat_material`  | A material of one flat colour, with roughness, metallic and an optional glow. Asking for a name again returns the same. |
| `palette`        | A flat material for each entry of a dict of colours, by name.                                                           |
| `brass`          | Polished brass that still reads as brass with no environment map.                                                       |
| `glazed_ceramic` | A soft body colour under a clear glossy glaze, so one lamp leaves a sharp highlight.                                    |
| `color`          | The linear colour of a hex colour, for values you set yourself.                                                         |

### Shapes

| Helper        | What it makes                                                                                                                                    |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `box`         | A box standing on its location, with an optional bevel that keeps the flat faces flat.                                                           |
| `cylinder`    | An upright cylinder or cone, with an optional bevel on its rims.                                                                                 |
| `sphere`      | A sphere, or an ellipsoid with `scale`.                                                                                                          |
| `extrude`     | A solid from a 2D outline pushed out along an axis: a book cover, a bracket, a rounded slab.                                                     |
| `lathe`       | A body turned around the z axis from a profile of (radius, height) points: a mug, a vase, a dome.                                                |
| `tube`        | A round tube along 3D points, open with end caps or closed into a ring: a handle, a cable.                                                       |
| `page_block`  | The pages of a book or notebook: a block whose edges carry fine, evenly spaced page lines as real geometry, plain on the side where it is bound. |
| `mesh_object` | A mesh object from raw vertices and faces.                                                                                                       |
| `empty`       | An empty: the root of a piece, or a pivot.                                                                                                       |

### Outlines and profiles

| Helper         | What it returns                                                                                                         |
| -------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `rounded_rect` | The outline of a rectangle with some or all corners rounded, for `extrude`.                                             |
| `fillet`       | A line of 2D or 3D points with its corners rounded by arcs: a lathe profile's foot and lip, the bends of a tube's path. |

### Shading and cutting

| Helper        | What it does                                                                                                                                               |
| ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crisp_edges` | Shades an object smooth and keeps the edges above an angle crisp, with no bevel: a rounded spine shades as one surface and the ends of a cover stay sharp. |
| `bevel`       | Rounds the edges above an angle and keeps the flat faces flat. For boxes and rims.                                                                         |
| `boolean`     | Cuts one object out of another, adds it or keeps what they share, then shades the result again.                                                            |

### Putting a piece together

| Helper       | What it does                                                                                                                |
| ------------ | --------------------------------------------------------------------------------------------------------------------------- |
| `join`       | Joins mesh objects into the first one. Every face keeps its own material, and a mirrored piece is turned to face out again. |
| `finish`     | The last step for a piece: applies its modifiers and parents it, standing where it is. It changes no material slot.         |
| `parent_to`  | Parents an object under another and leaves it standing where it is.                                                         |
| `set_origin` | Moves an object's origin without moving its geometry: the pivot of a lid.                                                   |
| `ground`     | Moves a piece so its bottom centre is at its root's origin, where Maquette expects a model's origin.                        |
| `bounds`     | The lowest and highest corner of a piece, in world space.                                                                   |
| `triangles`  | How many triangles a piece has.                                                                                             |

Keep helpers of your own in a file of your own that imports this one. An edited copy of `maquette_helpers.py` shows as edited on the banner and stops updating until you ask for the new one.

## The example

`built/maquette_example.py` ships next to the helper file. It builds a desk set that uses every helper once: a tray, two hardbacks with page lines and a ribbon marker, a mug and a brass desk bell with a button of its own.

Add it as a built asset to see it on the Board, with `built/maquette_helpers.py` as its helper. Then copy the parts a prop of yours needs. A few things it shows:

- A book cover is one outline pushed out along the spine, with no bevel. The rounded spine shades smooth and the ends stay crisp, so nothing leaves hairlines.
- The mug is one profile from the centre of its base, up the outside, over the lip and down the inside. `fillet` rounds the foot, the lip and the floor.
- The pages, the cover and the ribbon are joined into one mesh and keep their four materials.
- The bell's button is a child object with its origin where it meets the dome, so a page can press it.

## Updates

The helper file and the example are starters. When a newer Maquette ships a newer helper file, the Pipeline view offers it like any other starter. Updating it changes what the built assets that list it are made from, so each of them says Script changed and builds again before the next full run. An asset that does not list the file is not touched, and neither is a helper file of your own.

## For agents

`add_built_asset` adds one and builds it, `list_built_assets` reads each one's files, state and last build, and `rebuild_built_asset` builds one again or every new or changed one. `get_starters` and `update_starters` read and update the helper file with the other starters. All are in [MCP tools](/docs/agents/mcp-tools/).

To rebuild a generated model as a built asset and put it in that model's place, the `maquette-built-rebuild` [agent skill](/docs/agents/skills/) carries the whole recipe: the rules for the script, at least three render and fix rounds against the ref, and a side by side before anything ships.
