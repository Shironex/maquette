---
title: Render styles
description: Turn a model into a blueprint, toon, clay or shaded picture, a turntable loop or a contact sheet in headless Blender, with no credits spent.
sidebar:
  label: Render styles
---

Render styles makes pictures of a model for a README, a store page or a devlog. Headless Blender makes every one, through the same runner as the pipeline. No credits, and nothing leaves your machine. Files go to `exports/renders/<asset>/` in the project (or the workspace, for library models).

## The Render tab

The Inspector's Render tab works on one asset. Pick a style and an asset on the left, a camera above the picture and the settings on the right. The preview is rendered small and cached, so going back to settings you have seen answers at once. Render writes the full size file.

| Style         | What you get                                                                                           | How                                 |
| ------------- | ------------------------------------------------------------------------------------------------------ | ----------------------------------- |
| Blueprint     | Lines on blue paper with a grid, height and width marks and a title block. Hidden lines can be dashed. | Workbench, Freestyle lines          |
| Toon          | The model's own colours, flat, with ink lines                                                          | Workbench, Freestyle lines          |
| Clay          | One colour with cavity shading, to judge the shape alone                                               | Workbench studio light              |
| Shaded        | As built, on a floor, in a studio light                                                                | Cycles at a low sample count        |
| Turntable     | One full turn in any of the four looks                                                                 | See below                           |
| Contact sheet | Eight angles on one PNG                                                                                | Workbench or Cycles, then one sheet |

A compressed model renders from its uncompressed export, the same way a thumbnail does. A model nothing can read says so.

## Turntable

A loop is a WebP by default; GIF and MP4 are one click away. WebP needs nothing beyond Blender. GIF and MP4 need ffmpeg (Settings, [Toolchain](/docs/views/settings/)).

The GIF rules are fixed, not settings:

- One palette for the whole loop, built from every frame, so colours do not flicker.
- 20 fps, because GIF counts delays in hundredths of a second and 5/100 s is exact. Other rates round and drift.
- Bayer dithering at scale 5, because an ordered pattern compresses better than error diffusion.
- A crop box around the moving area. Pixels outside it stay still and cost almost nothing.

The sheet shows sample frames, the moving area and a size estimate against a 2 MB target. A GIF over the target is still written, and the Render tab offers Crop to the model and Save as WebP.

## Many at once

Select assets on the [Board](/docs/views/board/) or models in the [Library](/docs/views/library/), then Render. Each row has its own styles; Apply to all adds one to every row. Toon is left out for a model with no texture, and the sheet says so. Batch renders can also make one contact sheet of all the models, in columns with names under each.

Renders run one at a time beside the pipeline. You can close the sheet: the status bar keeps count. Pause stops after the file running now, Go on picks up from the next file, also after Maquette was closed.

## When a render goes wrong

Each problem says what to do next.

- **Blender or ffmpeg was not found:** Open Toolchain goes to the setting.
- **A GIF over 2 MB:** crop to the model, drop the width, or save a WebP loop.
- **Toon on a model with no texture:** clay and blueprint still work.
- **The line pass ran out of memory:** Try at 2048 px, or raise Edges from crease.

Agents can do all of this too: see [`render_styles`](/docs/agents/mcp-tools/#render_styles).
