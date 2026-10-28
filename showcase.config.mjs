// README images for Maquette, captured with @noctcore/showcase-kit from the
// web UI in mock mode. Regenerate them with `pnpm showcase`.
//
// The capture runs the React UI alone, without Tauri, Blender or any provider:
// outside Tauri a dev build answers every command from the in-memory backend
// in apps/web/src/ipc/mock, so the views show the Frostveil sample workspace
// that `pnpm dev:web` and `pnpm visual` use. Each shot opens one mock scenario
// (`?scenario=idle&library=demo` and so on) in a fresh page load.
//
// Determinism: the clock is frozen and the time zone pinned, every request
// outside the dev server is blocked, and the kit itself disables animations
// and waits for fonts. Two things stay live: the Inspector's frame time is
// measured, and the Inbox preview turns slowly on its turntable, so those two
// images can differ a little between runs.
//
// The 3D views render with the GPU (ANGLE on Metal on macOS, Direct3D 11 on
// Windows); the software renderer draws the same pictures but reports a few
// frames a second.
//
// Outputs:
// - showcase-out/raw/<id>.png: plain captures (gitignored).
// - assets/showcase/<id>.webp: the framed README images.
// - assets/showcase/hero.webp: the README banner (`pnpm showcase:hero`).
// - SHOWCASE_PORTFOLIO_DIR=<dir> adds 1920x1080 portfolio images there.
import { defineConfig } from '@noctcore/showcase-kit';

const PORT = 15196;
const ORIGIN = `http://localhost:${PORT}`;

/** Friday 2026-10-23, 20:40 in Warsaw: every "ago" in the mock counts from here. */
const FROZEN_NOW = new Date('2026-10-23T18:40:00Z');
const TIME_ZONE = 'Europe/Warsaw';

const BACKGROUND = { type: 'gradient', from: '#b8643f', to: '#16130f', angle: 135 };

/** Real GPU rendering for the three.js views, per platform. */
function gpuArgs() {
  const angle = { darwin: 'metal', win32: 'd3d11' }[process.platform];
  return angle ? [`--use-angle=${angle}`, '--enable-gpu', '--ignore-gpu-blocklist'] : [];
}

/**
 * Open one mock scenario on one view from a fresh page load, wait for `text`,
 * run `then` and park the pointer where it hovers nothing. Going through
 * about:blank reloads even when only the hash differs from the last shot.
 */
function view(query, hash, text, then) {
  return async page => {
    await page.goto('about:blank');
    await page.goto(`${ORIGIN}/?delay=300&${query}#/${hash}`);
    await page.getByText(text).first().waitFor({ timeout: 60000 });
    if (then) await then(page);
    await page.mouse.move(1439, 999);
  };
}

export default defineConfig({
  name: 'Maquette',
  slug: 'maquette',
  target: {
    mode: 'url',
    url: `${ORIGIN}/`,
    start: `pnpm --filter @maquette/web exec vite --port ${PORT} --strictPort`,
    readyTimeoutMs: 120000,
    reuseExisting: false,
  },
  ready: '#root > *',
  viewport: { width: 1440, height: 1000 },
  deviceScaleFactor: 2,
  colorScheme: 'dark',
  browser: { args: gpuArgs() },
  timeouts: { shotMs: 90000 },
  setup: async ({ page, context }) => {
    await context.route(
      url => !url.href.startsWith(ORIGIN) && !url.href.startsWith('data:'),
      route => route.abort()
    );
    await page.clock.setFixedTime(FROZEN_NOW);
    const cdp = await context.newCDPSession(page);
    await cdp.send('Emulation.setTimezoneOverride', { timezoneId: TIME_ZONE });
    await page.reload({ waitUntil: 'load' });
  },
  shots: [
    {
      id: 'board',
      title: 'Board',
      caption: 'Board: every asset of a project on its way from concept to ship.',
      nav: view('scenario=idle&library=demo', 'board', 'Update to v3'),
      delayMs: 1200,
    },
    {
      id: 'lab',
      title: 'Lab',
      caption: 'Lab: concept images from the Codex CLI, starred and picked as refs.',
      nav: view('scenario=idle&library=demo', 'lab', 'Frozen lake arena'),
      delayMs: 1200,
    },
    {
      id: 'inbox',
      title: 'Inbox',
      caption: 'Inbox: each new model checked for triangles, textures, rig and origin.',
      nav: view('scenario=idle&library=demo', 'inbox', 'Automatic checks'),
      delayMs: 3000,
    },
    {
      id: 'pipeline',
      title: 'Pipeline',
      caption: 'Pipeline: headless Blender build, bake, export, compress and ship.',
      nav: view(
        'scenario=running&pace=60&library=demo',
        'pipeline',
        'Run 25 · step 3 of 6',
        async page => {
          // Keep the edited export script, as most projects would, so its notice goes.
          await page.getByRole('button', { name: 'Keep mine' }).click();
        }
      ),
      delayMs: 1500,
    },
    {
      id: 'inspector',
      title: 'Inspector',
      caption: 'Inspector: two versions side by side, with budgets and compression.',
      nav: view('scenario=idle&library=demo', 'inspector', /WebGL 2 · \d+ fps/),
      delayMs: 3500,
    },
    {
      id: 'motion',
      title: 'Motion',
      caption: 'Motion: rigs, clips and what each one weighs on the page.',
      nav: view(
        'scenario=idle&motion=demo&library=demo',
        'motion',
        '119 bones · 2 clips',
        async page => {
          await page.getByText('119 bones · 2 clips').click();
          // Stop the clip on its first frame, so the pose is the same every run.
          await page.getByRole('button', { name: 'Pause' }).click();
          await page.getByRole('button', { name: 'Go to In' }).click();
        }
      ),
      delayMs: 3500,
    },
    {
      id: 'timeline',
      title: 'Timeline',
      caption: 'Timeline: camera stops, object states and paths, keyed to the scroll.',
      nav: view(
        'scenario=idle&timeline=demo&page=running&library=demo',
        'timeline',
        'Scroll timeline'
      ),
      delayMs: 2500,
    },
    {
      id: 'page',
      title: 'Page',
      caption: "Page: the site's own dev server, measured at every camera stop.",
      nav: view('scenario=idle&page=running&library=demo', 'page', 'Current ship vs this run'),
      delayMs: 2500,
    },
    {
      id: 'library',
      title: 'Library',
      caption: 'Library: finished models reused across projects, no credits spent.',
      nav: view('scenario=idle&library=demo', 'library', 'Versions · 2'),
      delayMs: 1200,
    },
    {
      id: 'agents',
      title: 'Agents',
      caption: 'Agents: Claude Code or Codex over MCP, with paid steps waiting for you.',
      nav: view('scenario=idle&agents=active&library=demo', 'board', 'Send Rune obelisk to Tripo?'),
      delayMs: 1200,
    },
  ],
  frame: {
    // The capture already shows the app's own title bar, so no second window frame.
    style: 'none',
    theme: 'dark',
    background: BACKGROUND,
    padding: 72,
    radius: 14,
    shadow: true,
    maxWidth: 1600,
  },
  outputs: {
    raw: 'showcase-out/raw/{id}.png',
    readme: 'assets/showcase/{id}.webp',
    ...(process.env.SHOWCASE_PORTFOLIO_DIR && {
      portfolio: {
        dir: process.env.SHOWCASE_PORTFOLIO_DIR,
        size: [1920, 1080],
        format: 'webp',
        thumbnail: 'board',
        gallery: false,
      },
    }),
  },
  hero: {
    tagline: 'Concept art in, optimised 3D for the web and games out.',
    logo: 'apps/desktop/src-tauri/icons/icon.png',
    shots: ['board', 'lab', 'page'],
    output: 'assets/showcase/hero.webp',
    background: BACKGROUND,
    theme: 'dark',
  },
});
