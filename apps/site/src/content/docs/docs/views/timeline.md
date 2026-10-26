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

Pick a pill, a block or a dot to edit it in the side panel. A change can switch between a camera stop and a scroll range; a timed change is not built yet. Its easing and length are the Motion view's transition. The panel's checks warn about a change that changes nothing, two ranges of one prop that overlap, an explode track that never moves, and a path that never fills.

## Saving and the site

Every edit saves into `layout.json` a moment later: the changes as the prop's stop and scroll triggers (the same triggers the Motion view lists), and the explode and energize tracks under `timeline`. Click and hover triggers stay with the Motion view.

Ship copies the layout into the site whole. The site plays the stop changes, the scroll ranges, the explode tracks and the energize tracks as the page scrolls. With reduced motion on, the site shows its posters instead of the 3D scene.
