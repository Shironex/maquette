---
title: Page view
description: See the project's website with its models live in it, at phone, laptop and desktop width.
sidebar:
  label: Page
---

The Page view shows the project's website with its models live in it, the way a visitor sees it. It runs the site's dev server itself and stops it when you leave the view or quit. How it picks the command and how the frame proxy and bridge work is on [Website projects](/docs/start/website-projects/).

## Modes

- **Preview:** the page in a browser frame at Phone (390 x 844), Laptop (1280 x 800) or Desktop (1440 x 900), scaled to fit. The scroll scrubber lists the page's camera stops. Drag it, step it or play it and the page scrolls along. The side shows the frame time the page measures, the scene on the page against the scene budget, GPU memory and draw calls against the device budget, and the current ship against this run.
- **Compare:** the current ship, live, beside this run's poster for the current stop, with its numbers and what changed. With poster adds the page without WebGL.
- **Devices:** the same page at all three widths at once, each following the scrubber with its own frame time.
- **Page concept:** an `.html` file inside the workspace, such as one the Lab made, shown as it is with no dev server.

Open in browser opens the page on the dev server itself. Without a site repo, with no dev script, or when the server fails, the frame says why and offers Settings, Try again or a page concept.
