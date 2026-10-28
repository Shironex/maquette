---
title: Inspector
description: Look at a model the way the page will, compare builds, tweak materials and place models and camera stops in the scene.
sidebar:
  label: Inspector
---

The Inspector opens a project's models in the same studio as the Inbox preview, with a compare slider that splits one canvas down the middle. Compare the raw export against the shipped build or the site's copy, or two versions of one asset. Both halves share the camera. Shaded, Colour by part, Wireframe and Lightmap pick how they draw.

## Side panel

The side panel lists the selected model's triangles, textures with size and codec, a GPU memory estimate, the transfer size and its compression preset. Under Compression, each map group (color, normal, data) has its own ETC1S or UASTC switch with what that group's maps weigh as built. Budget bars come from the workspace budgets and an optional `budgets.json` in the project. The frame time graph measures the renderer against the 60 fps line.

## Material

The Material section tweaks the selected model per asset: a tint, roughness (set to a value or multiplied), metalness and emissive strength. Applies to picks every material or one by name. The view previews edits at once and marks them Not applied yet. Apply tweaks saves them to `materials.json`, and [Compress](/docs/pipeline/compress/) applies them to the shipped model on the next run.

## Scene layout

The Layout tab places each asset's newest model and the camera stops in one 3D canvas. Click a model and drag the gizmo: Move, Turn and Scale (W, E, R), snapping to 5 cm and 15 degrees unless the magnet is off. The side panel takes exact numbers. Undo and redo cover every change, F frames every model and Cmd+S saves. All keys are on the [shortcuts](/docs/views/shortcuts/) page.

Camera stops are added where the editor camera stands, named, reordered, deleted, and each has a field of view. Up to 24 stops. Save layout writes `layout.json` in the project. The next pipeline run builds the scene, bakes it and renders one poster per stop from it.

The first view is what the first stop loads before the visitor moves. Its size is measured against its own budget, First view target under Settings (3 MB by default).

## Parts

The Parts tab lists a model's parts by the names they ship as: select them in the view, rename, merge and hide them, and put the model before and after its split side by side. See [Parts](/docs/views/parts/).

## Paths

The Paths tab draws cables, pipes and trails through the scene, anchored to models if you like, that the site lights up as the page scrolls. See [Paths](/docs/views/paths/).

## Game projects

A [game project](/docs/start/game-projects/) has no scene to lay out, so its Inspector shows Compare, Budget and Parts instead of Layout and Paths. Budget lists every model against its folder's budget, changes each folder's numbers and adds the fix-ups for the models over.
