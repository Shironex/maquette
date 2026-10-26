---
title: Paths
description: Draw cables, pipes and glowing trails through the scene, and light them up as the page scrolls.
sidebar:
  label: Paths
---

The Paths tab in the Inspector draws curves through the scene. The site shows each one as a tube that lights up from its first point as the page scrolls: a cable from a trench to a panel, a pipe along a wall, a trail across the ground.

## Drawing a path

New path adds a path one metre long across the middle of the scene. A path is a list of points in metres, Y up, the same space as the layout. Smooth joins them with a curve through every point, Straight with straight runs.

- **Select** (`V`): click a point to pick it and drag the gizmo to move it, or click a tube to pick its path.
- **Add points** (`A`): each click on a model or the ground adds a point after the picked one, or at the end.
- `Delete` removes the picked point; a path keeps at least two. `F` frames the scene, `Escape` lets go of the point, and the magnet snaps to 5 cm.

The points table under the view takes exact numbers, shows how far along the path each point is, and adds a point at the end. The side panel sets the name, colour, glow colour and width. Up to 32 paths of 2 to 128 points each.

## Anchors

Either end can follow a model. Pick one under Anchors and that end moves to where the model stands. From then on, when the model moves in the Layout tab or an agent moves it, the end moves by as much, so a cable stays plugged in. Only the move follows: turning or scaling the model leaves the end where it is. Adding a point past an anchored end, or removing that end's point, lets it go.

## Energize

Light it as the page scrolls gives the path an energize track: how much of it is lit at camera stops, from 0% (dark) to 100% (lit from the first point to the last). A new track is dark at the first stop and lit at the last. Edit the keys and the easing in the [Timeline](/docs/views/timeline/), where the path gets its own lane. The project needs a camera stop first.

## On the site

Every edit saves into `layout.json` half a second later, the paths under `paths` and their tracks under `timeline.paths`. Ship copies the layout into the site whole. The site builds each path as one tube mesh with 8 sides and a ring about every two widths, drawn in one draw call, and a shader fills it with the glow colour up to the lit amount, with a soft front and a bright head while it fills. The side panel shows each path's triangles and what it adds to `layout.json`, a few hundred bytes, so paths are not counted in the scene budget. With reduced motion the site shows its posters, and the posters do not draw the paths yet.

## Not built yet

Draw on surfaces, end caps, a glow strength, and the energize speed and direction show in the editor as not built yet. The fill always runs from the first point to the last, paced by the camera stops.
