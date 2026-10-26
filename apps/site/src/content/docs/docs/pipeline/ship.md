---
title: Ship
description: Copies the compressed models and posters into your site repo and writes the scene data.
sidebar:
  label: Ship
---

Step 6 of 6 in a new project. Kind: `shell`. Command: `node`. Script: `tools/ship.mjs`.

## What it does

It copies `web/dist/*.glb` and the posters into the project's site repo and writes the site's scene data when the site asks for it. Without a site repo it says so and passes. Where things go is set in [`maquette.site.json`](/docs/start/website-projects/#maquettesitejson).

Ship has no outputs inside the project, so it runs every time.

## Safe by design

- It checks every folder before it copies anything, and stops before an empty layout could wipe the site's scene.
- With a data file, it copies only the models the data lists, so a leftover in `web/dist` never reaches the site.
- It keeps `web/ship-record.json` with the files it shipped into each site folder. A file it shipped last time and no longer ships is removed from the site and logged. Every other file in the site is left alone.
- It never commits.

:::caution[Ask before shipping]
With Ask before shipping on (the default), a run an agent starts over MCP stops before Ship and the app shows Approve or Deny. Until you answer, `run_pipeline` and `get_run` report `waiting for approval`.
:::
