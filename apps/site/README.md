# Maquette landing page

The public page for Maquette: a 3D sculptor's workbench at night that the camera tours as you scroll, one stop per pipeline stage (Concept, Refs, 3D model, Blender, Web, Ship), followed by the sections about the app.

Astro builds it to static HTML. The 3D scene is one React island (React Three Fiber on three.js) that only desktop class devices load; phones and browsers without WebGL 2 get the poster version, with no 3D download at all.

## Run and build

```sh
pnpm dev:site                          # http://localhost:15292/maquette/
pnpm build:site                        # static output in apps/site/dist
pnpm --filter @maquette/site preview   # serve the build
pnpm --filter @maquette/site typecheck # astro check
```

`pnpm test` at the root runs the site tests with the others, and `pnpm gate` builds the site.

GitHub Pages serves the site under `/maquette/`, the path of `SITE_URL` (`src/lib/site-url.ts`), so that is the `base` in dev and preview too. A root path in a page or the scene goes through `withBase` (`src/lib/with-base.ts`); the docs Markdown keeps root links (`/docs/...`) and `src/lib/base-links.ts` puts them under the base. `.github/workflows/pages.yml` builds and deploys the site on a push to `main`.

## How the page works

- `src/pages/index.astro` puts the sections together; each lives in `src/components/`. The copy is in `src/content/copy.ts`.
- The tour (`Tour.astro`) pins a full window stage (the canvas, or a poster per stop without it) while one screen tall section per stop scrolls over it (`TourStop.astro`): the hero, then Concept to Ship, each stop's copy on the side its framing leaves clear. `src/scene/scroll.ts` measures the sections' edges once and again on resize and turns the scroll position into fractional progress (0 on the hero, 1 on Concept, up to 6 on Ship): a stop is reached as its section's top meets the top of the window, with its copy centred on screen. The copy scrolls with the page and dims only near the window's edges. After Ship the stage scrolls away with the last section. `src/scene/startTour.ts` runs the page side: the stop rail along the bottom of the stage (`TourRail.astro`, a button per stop), the posters and the island.
- The island reads the same progress, eased a little behind the scroll, and flies the camera through the bench (`src/scene/path.ts`). Each stop in `src/scene/cameras.ts` has its framing, its move from the design's camera board (dolly, truck, orbit, push in, pull back) and the poses it flies through on the way in (`via`). One centripetal curve runs through every camera position and one through every look target (`track.ts`), and a monotone timing (`spline.ts`) walks both: slow through each stop's move while its copy is on screen, fast on the flights between, never stopping.
- At rest the scene keeps living: the camera breathes a few millimetres and leans after a mouse pointer, the turntable idles round, the lamp's filament wavers and dust drifts in its light (`Motes.tsx`). Frames render continuously while the canvas is on screen and the tab is shown, and stop otherwise. The turntable's turn and the wireframe sweep also follow the scroll.
- The stage shows the poster of the current stop first. After the page has loaded, on a fine pointer at 1024 px wide or more with WebGL 2 and without reduced motion, the script imports `src/scene/mount.tsx`, which renders the scene. The poster stays until the scene says it is ready, and comes back if WebGL fails or the context is lost.
- With reduced motion, and on phones, there is no 3D download: reduced motion keeps the poster per stop, swapped without a fade as the sections pass; phones get the hero poster and the stops are left to the pipeline section.
- In Maquette's Page view, `src/scene/bridge.ts` follows the stop scrubber: after the app says hello it answers with the stops (the hero, then the tour), scrolls to a fractional stop on `goto` with `scrollForProgress` (`src/scene/progress.ts`), reports where the scroll is and the frame time about once a second. `?maquette-poster=1` keeps the poster version. The messages are listed in the root README under Page view; outside a frame the bridge stays off.
- Models load in the order the current stop needs them (`loadOrder` in the manifest), three at a time, through one GLTF loader with meshopt and KTX2. The Basis transcoder comes from the `three` package: the dev server serves it and the build writes it to `dist/basis/`, both under the base.

## Manifest

`src/scene/manifest.ts` puts the scene together; units are metres, Y is up and the bench top is at `BENCH_TOP` (0.9).

- The props and the camera framings come from the Maquette project. Its Ship step writes `src/scene/shipped.json` (where `maquette.site.json` points it): each asset's shipped file, the project's `layout.json` and `materials.json`. `fromLayout()` in `src/scene/layout.ts` turns the layout into model entries and stops: the layout's stops map in order onto the site's (the first is the hero frame, then Concept to Ship), a stop's `models` list says which stops see a model (`null` means every model), and `lift` raises everything from the layout's desk top (y = 0) onto the bench.
- The site adds what the layout does not know: the set pieces built for the bench (workbench, back wall, paper, tracing sheets, crumbs) in `manifest.ts`, each prop's extras in `src/scene/props.ts` (anchor, spin, sweep, shadows, the laptop screen, the turntable's iron band) and the camera's motion between stops in `src/scene/cameras.ts` (move, the poses on the way in, the copy's side). A stop the layout lacks keeps the last framing it has.
- Colours, roughness, metal and glow are the project's material tweaks. The pipeline bakes them into the shipped files, so the site never applies them again; `shipped.json` keeps a copy as a record.
- A model Ship exports already stands where the layout puts it (`inPlace`): the site moves it back by its placement and turns it about that point. The props on the bench today were prepared by hand, so their entries say `"inPlace": false` and stand on the bottom centre of their bounds.
- `shipped.json` is left as Ship writes it (JSON.stringify), so it is out of prettier. Ship refuses an empty layout, so set the layout in Maquette (the `layout` of this file is a valid `layout.json` in desk coordinates) before the first ship.

## Shipping from Maquette

Set the project's site repo to `apps/site` in Settings > Project and run the pipeline; the Ship step waits for approval in the app, then copies the models into `public/scene/models` and writes `src/scene/shipped.json`. **Preview on site** in the Pipeline view starts this site's dev server (`pnpm dev`, port 15292) and opens it. Then rebuild and render the posters again (below); deploying stays a separate step.

A raw model that does not come through the pipeline goes through the same chain the pipeline uses:

```sh
npx @gltf-transform/cli@4.5.1 etc1s in.glb tmp.glb --resize 512
npx @gltf-transform/cli@4.5.1 meshopt tmp.glb out.glb --level medium
```

Use `--resize 1024` for the bench top and the back wall.

## Posters

`public/scene/posters/` holds one frame per stop (1920 x 912, 960 x 456 and a 192 x 108 thumbnail) and the phone poster. They are rendered from the site's own scene:

```sh
pnpm build:site
PLAYWRIGHT=/path/to/playwright/index.mjs pnpm --filter @maquette/site posters
```

The script opens the built page in headless Chromium with `?capture`, which hides everything but the canvas, holds the camera on each stop's exact framing (no breathing, pointer or idle motion) and encodes WebP in the page. Run it after any change to the stops, the manifest, the models or the lighting, and after a ship. The posters Maquette renders in Blender show the project's scene without the bench's set pieces, so `maquette.site.json` leaves them out (`"posters": null`) and the tour keeps its own.

## Releases

The Download section reads the `release` object in `src/content/copy.ts` (version, date, file names, byte sizes). A new release is an edit to that object.

## Placeholders

- The laptop screen (`public/scene/laptop-screen.webp`) is a mock mode capture of the Board; swap in a capture of a real project.
- The app screenshots in `public/scene/screens/` are mock mode captures too, of the v0.2.0 web UI (`apps/web` with `?scenario=...`), whole windows or one dialog each. The sample project in the mock fixtures carries real names, so they were swapped for neutral ones (Winter Spire, Aria) when the shots were taken; retake them the same way after a UI change: `pnpm --filter @maquette/site screens` (or `screens board,inbox` for some ids) runs `scripts/capture-screens.mjs`, which needs no build and writes the WebP files in place, the laptop screen included. Render the posters again afterwards (see Posters), since the scene shows the laptop screen.

## Showcase

The "On a real project" section (`src/components/Showcase.astro`, data in `src/content/showcase.ts`) shows files from a game project made in Maquette, in `public/scene/showcase/`: its concept sheet, object refs, model thumbnails and view refs, scaled down to WebP, plus two renders of its character GLBs (a still and a 2.7 s loop as H.264 MP4 with a WebP poster). The loop is fetched and played only while it is on screen, and never with reduced motion. Captions call it "a game project", not by name.

## Download links per platform

`src/lib/platform.ts` guesses the visitor's platform from `navigator.userAgentData.platform`, else the user agent. Its source runs inline in the head (Base.astro, and Starlight's `head` in astro.config) and sets `data-os` on the root element before the first paint; `src/styles/os.css` then shows the matching links from `DownloadLink.astro` and the Download section. macOS and Windows get their file; Linux, phones and pages without JavaScript get the neutral Download that leads to both buttons. The Mac label names Apple silicon; the header, short on space, says only "Mac (Apple silicon)".

- The props on the bench were prepared by hand (`"inPlace": false` in `shipped.json`) until the first ship from Maquette.
- The finished island keeps its 2048 color texture (the Web stop is a close up of it); a 1024 copy would save about 0.5 MB.

The artwork under `public/scene/` is reserved; see the ARTWORK section of the root LICENSE.
