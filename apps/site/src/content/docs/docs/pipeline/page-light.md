---
title: The page light
description: The one light of a built asset's thumbnail and of the standard views, so a colour that reads right in a preview reads right on the page.
sidebar:
  label: The page light
---

A model built by a script is judged by eye: its flat palette colours either read right or they do not. So the previews it is judged by are lit the way its page lights it. One rig holds that light, `preview_rig.py`, and every render that shows a model for judging uses it.

## Where it is used

- The Board thumbnail of a [built asset](/docs/views/board/). The look is chosen per asset, not per version: once an asset is built, every version of it gets the page light, also one a provider made before. Every other asset keeps the studio thumbnail, and its cached pictures stay as they are.
- [`render_views`](/docs/agents/mcp-tools/#render_views), the standard views an agent compares with a ref.

## The numbers

The light is the one [Bake light](/docs/pipeline/bake-light/) bakes into a model, as [Build scene](/docs/pipeline/build-scene/) sets it up. The display is the reference page's renderer.

| What         | Value                                        | Where it comes from                         |
| ------------ | -------------------------------------------- | ------------------------------------------- |
| Sun          | strength 3.0, tilted 0.9 radians off the top | `tools/build_scene.py`                      |
| Even fill    | a grey world of 0.18, from every side        | `tools/build_scene.py`                      |
| Tone mapping | three.js ACES Filmic                         | the site scene's renderer (`Workbench.tsx`) |
| Exposure     | 1.15                                         | the same                                    |
| Output       | sRGB                                         | three.js' output colour space               |

A white surface gets 0.18 where the sun does not reach, 0.77 lying flat in the sun and 1.13 facing it. The sun comes from the front left, so the three quarter view shows a sunlit side and a fill side of every colour. The starter's own sun comes from the front right; only that side is mirrored.

The tone curve is three.js' own, applied to Blender's float render, so no Blender view transform (Standard, AgX, Filmic) stands between the preview and the page. A test holds the rig's numbers to `build_scene.py` and to the site scene, so they cannot drift apart unseen.

The studio thumbnail's key light, for comparison, puts 2.28 on a white surface and has no tone curve. Every pale colour clips to white under it, which is why a built asset no longer uses it.

## The swatch sheet

![Fifteen palette colours as tiles in the page light, each with a strip of the colour as written and its hex](/docs-media/page-light-swatches.png)

Each tile is a flat surface of that base colour lying in the sun, seen from straight above. The strip under it is the colour exactly as written, untouched by the light. Read a tile against its strip:

- Pale colours keep their hue and lose a little saturation: lavender `#c9b8e8` shows as about `#c7bdd5`, cream `#f3e9d2` as about `#dad6cd`.
- White stays under white, about `#dedede`, so there is room above it and nothing clips.
- Dark colours come out a touch lighter: a surface keeps a little sheen.

Render it again, or your own palette, with the real Blender:

```sh
cargo run -p maquette-core --example swatch_sheet -- sheet.png
cargo run -p maquette-core --example swatch_sheet -- mine.png --swatches "cover=#c9b8e8,pages=#f3e9d2"
```

`--report sheet.json` writes what each tile measures, and `--glb tiles.glb` the bare tiles, to show the same colours on a page.

## Side by side with the page

The sheet's tiles were built, baked and exported by the starter scripts, then shown the way [a page shows baked light](/docs/start/website-projects/#baked-light-on-the-page) through the site scene's renderer settings. The pale colours land within 2 of 255 per channel of the rig's render (lavender 196, 186, 211 on the page and 197, 187, 212 in the rig), and the darkest within 16, which is the sheen a light map does not carry.

## What the rig cannot know

A page may add lights of its own on top of the baked light. The reference page does: a warm desk lamp that puts up to 2.3 of red on a surface under it, which no neutral preview should copy. Its even fill, measured on the same tiles with the bake as ambient occlusion, is 0.20, beside the rig's 0.18. So the fill side of a preview is what such a page shows where its own key does not reach.

## Colours in a built script

A Principled BSDF's Base Color is linear, and a hex colour is sRGB. Convert it before you set it, or every colour comes out paler than written:

```python
def linear(hex_color):
    def channel(c):
        c = int(c, 16) / 255
        return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4

    h = hex_color.lstrip("#")
    return (channel(h[0:2]), channel(h[2:4]), channel(h[4:6]), 1.0)
```
