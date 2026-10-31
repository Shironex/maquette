---
title: Page view
description: See the project's website with its models live in it, at phone, laptop and desktop width.
sidebar:
  label: Page
---

The Page view shows the project's website with its models live in it, the way a visitor sees it. It runs the site's dev server itself and stops it when you leave the view or quit. How it picks the command and how the frame proxy and bridge work is on [Website projects](/docs/start/website-projects/).

## Modes

- **Preview:** the page in a browser frame at Phone (390 x 844), Laptop (1280 x 800) or Desktop (1440 x 900), scaled to fit. The scroll scrubber lists the page's camera stops. Drag it, step it or play it and the page scrolls along. The side shows the frame time the page measures, the scene on the page against the scene budget, GPU memory and draw calls against the device budget, and the current ship against this run.
- **Hotspots:** on one page, pins where the camera sees a prop with click or hover triggers from the Motion view, named by the state they go to. They sit on the prop's placement, exact at a stop and close between stops, and never take the page's pointer.
- **Compare:** the current ship, live, beside this run's poster for the current stop, with its numbers and what changed, headed Why this run is lighter or heavier once they can be weighed. With poster adds the page without WebGL, with the stop it shows and at what size the page swaps to 3D.
- **Devices:** the same page at all three widths at once, each following the scrubber with its own frame time.
- **Page concept:** an `.html` file inside the workspace, such as one the Lab made, shown as it is with no dev server. The address bar says Mock and the source list says Generated mock; only the scene numbers count there.

Open in browser opens the page on the dev server itself. **Stop** in the frame's address bar stops the dev server, and the bar says who started it when it was not the Page view (Preview on site or an agent). It is the same server Preview on site and the status bar show, so stopping it in any of them updates the others. Without a site repo, with no dev script, or when the server fails, the frame says why and offers Settings, Try again or a page concept.
