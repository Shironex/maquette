---
title: Timeline
description: Lay out what happens as the page scrolls, stop by stop, and scrub it on the page itself.
sidebar:
  label: Timeline
---

The Timeline runs along the page from top to bottom, with the project's camera stops as its columns. The site's stop sections are one window high each, so stop 1 of 5 sits at the top, stop 3 at 50% and stop 5 at the bottom. A project without camera stops has no columns yet: add them in the Inspector first.

Above the lanes is the project's page from its dev server, as the Page view shows it. Drag the ruler, press `[` and `]` to jump a stop, or `Space` to play the scroll at reading speed, and the page scrolls along through the page bridge.

## Lanes

- **Camera**: a stop at each one's share of the page, and the flight between them. Click a stop to jump there.
- **A prop's states**: every prop with states in the Motion view gets a lane. A pill is a change when the camera arrives at a stop; it plays the move once, timed by its transition. A block is a scroll range; the move scrubs with the page from its start to its end. The plus at a stop adds a change there, going on to the next state.
- **Explode**: Add track > Explode puts a model's parts on a track. A dot at a stop says how far apart the parts are there (0% together, 100% the full spread), and the page eases between the dots. Each part moves straight out from the model's centre.
- **Paths**: every path with an energize track gets a lane. A dot at a stop says how much of the path is lit there (0% dark, 100% lit from end to end), and the page eases between the dots. Add track > Energize gives a path a track, and New path... opens the Inspector's [Paths](/docs/views/paths/) tab to draw one.
- **Render style**: Add track > Render style fades the 3D scene into a look as the page scrolls: Blueprint, Toon or Clay. A dot at a stop says how far into the look it is (0% as built, 100% the whole look). The site draws it as a filter on the 3D layer, so it adds no draw calls. It is a live approximation; the [render styles](/docs/views/renders/) Blender makes are separate pictures.
- **Hero text**: Add track > Hero text fades the copy the site marks with `data-maquette-text`. A dot at a stop says how much shows there (0% hidden, 100% shown).

Pick a pill, a block or a dot to edit it in the side panel. A change can switch between a camera stop and a scroll range; a timed change is not built yet. Its easing and length are the Motion view's transition. The panel's checks warn about a change that changes nothing, two ranges of one prop that overlap, a range that finishes off screen, a track that never moves, and a path that never fills, and each card has a button that fixes it. A range finishes off screen when it runs past a stop whose model list, set by hand, leaves that prop out; its card ends the range there or slides it earlier at the same length. Stops that measure their view are not checked, since the app cannot tell what they see without the page. Nothing holds the camera for you: a stop's share of the page follows from the site's sections.

## Saving and the site

Every edit saves into `layout.json` a moment later: the changes as the prop's stop and scroll triggers (the same triggers the Motion view lists), and the explode, energize, render style and hero text tracks under `timeline`. Click and hover triggers stay with the Motion view.

Ship copies the layout into the site whole. The site plays the stop changes, the scroll ranges and all of these tracks as the page scrolls. With reduced motion on, the site shows its posters instead of the 3D scene.
