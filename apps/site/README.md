# Maquette landing page

The public page for Maquette: a 3D sculptor's workbench at night that the camera tours as you scroll, one stop per pipeline stage (Concept, Refs, 3D model, Blender, Web, Ship), followed by the sections about the app.

Astro builds it to static HTML. The 3D scene is one React island (React Three Fiber on three.js) that only desktop class devices load; phones and browsers without WebGL 2 get the poster version, with no 3D download at all.

## Run and build

```sh
pnpm dev:site                          # http://localhost:15292
pnpm build:site                        # static output in apps/site/dist
pnpm --filter @maquette/site preview   # serve the build
pnpm --filter @maquette/site typecheck # astro check
```

`pnpm test` at the root runs the site tests with the others, and `pnpm gate` builds the site.

## How the page works

- `src/pages/index.astro` puts the sections together; each lives in `src/components/`. The copy is in `src/content/copy.ts`.
- The tour (`Tour.astro`) is a tall section with a sticky stage, 130 % of a screen of scroll per move between stops. `src/scene/scroll.ts` turns the scroll position into fractional progress (0 on the hero, 1 on Concept, up to 6 on Ship) from a layout it measures once and again on resize; the page fades and drifts each stop's copy and fills the timeline with it, and the stop timeline buttons scroll to a stop.
- The island reads the same progress, eased a little behind the scroll, and puts the camera on one path through every stop (`src/scene/path.ts`): each stop plays its move from the design's camera board (`src/scene/cameras.ts`: dolly, truck, orbit, hold, push in, pull back) around its own framing, and the path runs on into the next stop. The turntable's turn and the wireframe sweep follow the scroll too.
- The stage shows the poster of the current stop first. After the page has loaded, on a fine pointer at 1024 px wide or more with WebGL 2, the script imports `src/scene/mount.tsx`, which renders the scene. The poster stays until the scene says it is ready, and comes back if WebGL fails or the context is lost.
- With reduced motion the camera cuts between stops, the copy swaps with them, nothing turns and the wireframe sweep holds still.
- Models load in the order the current stop needs them (`loadOrder` in the manifest), three at a time, through one GLTF loader with meshopt and KTX2. The Basis transcoder comes from the `three` package: the dev server serves it and the build writes it to `dist/basis/`.

## Manifest

`src/scene/manifest.ts` is the scene's single source: every model (file, position, rotation, scale, which stops see it, material override, extras); the camera of each stop and its move live beside it in `src/scene/cameras.ts`. Units are metres, Y is up and the bench top is at `BENCH_TOP` (0.9).

The placements and stops have the same shape as a Maquette project's `layout.json`, and `fromLayout()` in `src/scene/layout.ts` turns one into the manifest's `models` and `stops`:

- the layout's stops map in order onto the site's stops: the first is the hero frame, then Concept to Ship;
- a stop's `models` list says which stops see a model (`null` means every model);
- `assets` gives each asset id its shipped file and the site extras (anchor, material override, spin, sweep, screen);
- `lift` raises everything from the layout's desk top (y = 0) onto the bench.

The comment at the top of `layout.ts` documents the `layout.json` shape. To take a new layout from Maquette, read the project's `layout.json`, pass it through `fromLayout` with the assets the site ships, and replace `MODELS` and `STOPS` with the result (keep the set pieces and extras that are site only).

## Updating models from Maquette

The models live in `public/scene/models/`. Pipeline output (the project's `web/dist/*.glb`) is already compressed and copies over as is. A raw model goes through the same chain the pipeline uses:

```sh
npx @gltf-transform/cli@4.5.1 etc1s in.glb tmp.glb --resize 512
npx @gltf-transform/cli@4.5.1 meshopt tmp.glb out.glb --level medium
```

Use `--resize 1024` for the bench top and the back wall. Then set the entry's scale and position in the manifest, rebuild and render the posters again.

## Posters

`public/scene/posters/` holds one frame per stop (1920 x 912, 960 x 456 and a 192 x 108 thumbnail) and the phone poster. They are rendered from the site's own scene:

```sh
pnpm build:site
PLAYWRIGHT=/path/to/playwright/index.mjs pnpm --filter @maquette/site posters
```

The script opens the built page in headless Chromium with `?capture`, which hides everything but the canvas, and encodes WebP in the page. Run it after any change to the manifest, the models or the lighting.

## Placeholders

- The laptop screen (`public/scene/laptop-screen.webp`) is a mock mode capture of the Board; swap in a capture of a real project.
- The app screenshots in `public/scene/screens/` are mock mode captures too.
- The download buttons point at `#download` and the section says "Not released yet": the links, version and file size come with the first build.
- The finished island keeps its 2048 color texture (the Web stop is a close up of it); a 1024 copy would save about 0.5 MB.

The artwork under `public/scene/` is reserved; see the ARTWORK section of the root LICENSE.
