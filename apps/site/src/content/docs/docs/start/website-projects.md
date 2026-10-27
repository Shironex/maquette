---
title: Website projects
description: Ship a scene into a site repo, say where files go with maquette.site.json, and watch the site in the Page view.
sidebar:
  label: Website projects
---

A website project builds a scene for a site repo. The pipeline bakes light into each model, renders a poster per camera stop and ships compressed GLBs, the posters and the scene data into the repo. Deploying stays your step.

## Ship to a site repo

Pick the site repo when you create the project, or later in Settings. The [Ship](/docs/pipeline/ship/) step copies `web/dist/*.glb` and the posters into it. Without a site repo, Ship says so and passes.

## maquette.site.json

The site says where things go in `maquette.site.json` at its root. Folders are relative to the site and must stay inside it.

```json title="maquette.site.json"
{
  "version": 1,
  "models": "public/scene/models",
  "posters": null,
  "data": "src/scene/shipped.json",
  "preview": {
    "command": ["pnpm", "dev"],
    "url": "http://localhost:15292/"
  }
}
```

| Field     | What it does                                                                                                          |
| --------- | --------------------------------------------------------------------------------------------------------------------- |
| `models`  | The folder shipped GLBs go into.                                                                                      |
| `posters` | The folder for posters. `null` copies none.                                                                           |
| `data`    | The file Ship writes the scene data to. Leave it out for no data.                                                     |
| `preview` | The dev server the Page view and Preview on site run. `command` is a program and its args, `url` is where it answers. |

A site without the file gets `public/models` and `public/posters` and no data. **Ship settings** in the Pipeline view writes the folders for you, keeping `preview` and any other key. A site that places each model itself suits the [Hand-built site](/docs/pipeline/ship/#a-hand-built-site) style.

### The scene data

The data file lists each asset's shipped model, marked `inPlace` because the export keeps every model where the layout put it. It also carries the project's `layout.json` and `materials.json` as they are. Ship only writes it from a layout with placed models and camera stops, so an empty layout can never wipe the site's scene. With a data file, Ship copies only the models the data lists.

### Baked light on the page

A baked material ships its lightmap as the glTF emissive texture, sampled with `TEXCOORD_1`, and the material extras `{ "lightmap": <scale>, "lightmapUv": 1 }`. The map stores light divided by the scale. In three.js, a page moves it to a light map:

```js
material.lightMap = material.emissiveMap;
material.lightMapIntensity = scale * Math.PI;
material.emissiveMap = null;
material.emissive.set(0x000000);
```

A material without the extras has no bake and is lit live.

## The Page view

The [Page view](/docs/views/page/) shows the site with its models live in it. It runs the site's dev server itself on a free port. The command comes from Settings (Site dev command, where `{port}` stands for the port Maquette picks), else from `maquette.site.json`, else from the site's `package.json` scripts (`dev`, `start` or `serve`) run by the manager of the nearest lockfile. It runs as a program with its args, never through a shell.

**Preview on site** in the Pipeline view starts the same server and opens it in the browser. Nothing is built or deployed.

## The frame proxy

Most sites forbid being framed: `X-Frame-Options: DENY` or `Content-Security-Policy: frame-ancestors 'none'`. So the Page view does not load the dev server directly. It loads a small proxy Maquette runs on `127.0.0.1`, in front of that one dev server.

The proxy drops `X-Frame-Options` and only the `frame-ancestors` directive of the CSP, and leaves every other header and body as the site sent it. It streams responses, passes WebSockets through so hot reload keeps working, and refuses anything that is not on this machine. Nothing in your site changes.

## The bridge and its shim

The scrubber under the Page view talks to the page through window messages that all carry `maquette: 1`.

```js
// Maquette to the page
{ maquette: 1, type: 'hello', stops: [{ name: 'Porch' }] }
{ maquette: 1, type: 'goto', stop: 2.5, smooth: true } // 2.5 is halfway from stop 3 to 4

// The page to Maquette
{ maquette: 1, type: 'ready', stops: [{ name: 'Hero' }], features: ['goto', 'at', 'stats', 'poster'] }
{ maquette: 1, type: 'at', stop: 2.5, scroll: { top: 900, max: 3600 } }
{ maquette: 1, type: 'stats', frameMs: 9.4, p95Ms: 12.1, drawCalls: 86 }
```

A site does not need to add the bridge. The frame proxy adds a shim to every page it frames, a script it serves itself at `/__maquette/bridge.js`, so a CSP with `script-src 'self'` allows it. The shim scrolls to the element whose `id` is a stop's slug, or whose `data-maquette-stop` names it, and otherwise spreads the camera stops evenly over the page's scroll. It measures frame times, but draw calls stay estimated.

A page with its own bridge sets `window.__maquetteBridge = 1`, and the shim stays out of its way. With `?maquette-poster=1` in its address, a page shows its version without WebGL.
