---
title: Capture posters
description: Renders one poster per camera stop, for the page to show before WebGL loads.
sidebar:
  label: Capture posters
---

Step 5 of 6 in a new project. Kind: `blender`. Script: `tools/render_stops.py`.

## What it does

It renders one poster per camera stop into `web/posters/`. A stop is a camera with the `maquette_stop` custom property (its number sets the order) or a name starting with `stop_`.

A scene without stops gets one framed three quarter poster of the whole scene instead, named `overview.png`, and the log says so. The step always writes `web/posters/stops.json`, with each stop's name from the layout as its `label`, and removes the posters the previous run listed.

## Inputs and outputs

**Input:** `blender/scene_baked.blend`.

**Output:** `web/posters/stops.json`, next to the posters.

[Ship](/docs/pipeline/ship/) copies the posters into the site when `maquette.site.json` names a folder for them.
