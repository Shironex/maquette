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
import { rename } from './apps/site/scripts/screens-page.mjs';

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
    // The sample project's own names become the neutral ones the site shows,
    // in the app and in any page it frames (the Page view's site).
    for (const frame of page.frames()) await frame.evaluate(rename);
    await page.mouse.move(1439, 999);
  };
}

/** The site loads in its frame after the view; wait for it so it is renamed too. */
async function siteLoaded(page) {
  const site = page.locator('iframe').first().contentFrame();
  await site.getByText('A frozen lake you can scroll through.').waitFor();
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
      nav: view('scenario=idle&library=demo', 'board', 'Library has v3'),
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
          // The live log names the sample's character as it goes; let the step reach its last
          // line before the renaming runs, so no line arrives after it.
          await page.getByText(/exporting .+\.glb, 2 clips/).waitFor();
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
        /119 bones · \d+ clips/,
        async page => {
          await page.getByText(/119 bones · \d+ clips/).click();
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
        'Scroll timeline',
        siteLoaded
      ),
      delayMs: 2500,
    },
    {
      id: 'page',
      title: 'Page',
      caption: "Page: the site's own dev server, measured at every camera stop.",
      nav: view(
        'scenario=idle&page=running&library=demo',
        'page',
        'Current ship vs this run',
        siteLoaded
      ),
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
    {
      id: 'animations',
      title: 'Animations',
      caption: 'Animations: a library of clips, each one playing on its own skeleton.',
      nav: view(
        'scenario=idle&library=demo',
        'animations',
        '16 clips · newest first',
        async page => {
          await page
            .getByRole('complementary', { name: 'Selected clip' })
            .getByRole('heading', { name: 'Standing Dodge Left' })
            .waitFor();
        }
      ),
      delayMs: 2500,
    },
    {
      id: 'retarget',
      title: 'Use on model',
      caption: 'Use on model: library clips retargeted onto a rig, feet pinned.',
      nav: view(
        'scenario=idle&library=demo&motion=demo',
        'motion',
        /119 bones · \d+ clips/,
        async page => {
          await page.getByRole('button', { name: 'Add clip' }).click();
          await page.getByRole('menuitem', { name: 'Add from library' }).click();
          const steps = page.getByRole('dialog', { name: /^Add library clips to / });
          for (const name of [
            'Sword And Shield Run',
            'Sword And Shield Slash',
            'Sword And Shield Crouch',
            'Standing Dodge Left',
          ]) {
            await steps.getByText(name, { exact: true }).click();
          }
          await steps.getByText('Chosen · 4').waitFor();
          await steps.getByRole('button', { name: 'Next: bone map' }).click();
          await steps.getByRole('region', { name: /^(Bone map|Clip settings)$/ }).waitFor();
          await steps
            .getByRole('navigation', { name: 'Steps' })
            .getByRole('button', { name: /Clip settings/ })
            .click();
          await steps.getByRole('region', { name: 'Clip settings' }).waitFor();
          await steps.getByRole('button', { name: 'Retarget 4 clips' }).click();
          await page.getByRole('dialog', { name: /^Review the retarget on / }).waitFor();
          await page.waitForTimeout(2500);
          await page.getByRole('button', { name: 'Pin the feet and bake again' }).click();
          await page.getByRole('group', { name: 'Feet pinned' }).waitFor();
          await page.waitForTimeout(2500);
        }
      ),
      delayMs: 1500,
    },
    {
      id: 'avatar',
      title: 'Avatar',
      caption: 'Avatar: a rigged character to VRM 1.0, each step waiting for a yes.',
      nav: view('scenario=idle&avatar=pending&library=demo', 'avatar', 'Rest pose', async page => {
        const approval = page.getByRole('region', { name: 'Rest pose approval' });
        await approval.waitFor();
        await page.getByRole('img', { name: 'The after sheet' }).last().waitFor();
        // Bring the rest pose card and its before and after sheets to the top.
        await approval.evaluate(el => el.scrollIntoView({ block: 'start' }));
        await page.waitForTimeout(500);
      }),
      delayMs: 1500,
    },
    {
      id: 'parts',
      title: 'Parts',
      caption: 'Parts: a model split into named parts, coloured and picked in the Inspector.',
      nav: view('scenario=idle&parts=demo&library=demo', 'inspector', 'Parts', async page => {
        await page.getByRole('tab', { name: 'Parts' }).click();
        const list = page.getByRole('list', { name: 'Parts' });
        await list.waitFor();
        await list.getByRole('button', { name: 'cage' }).click();
        await list.getByRole('button', { name: 'crystal' }).click({ modifiers: ['Shift'] });
        await page.getByText('cage and crystal selected').waitFor();
        await page.waitForTimeout(1500);
      }),
      delayMs: 2000,
    },
    {
      id: 'refs',
      title: 'Refs by view',
      caption:
        'Refs by view: refs tagged front, back and sides, with a warning when they disagree.',
      nav: view(
        'scenario=configured&lab=refs&library=demo',
        'board',
        'Guardian statues',
        async page => {
          await page.getByRole('button', { name: 'More for Guardian statues' }).click();
          await page.getByRole('menuitem', { name: 'Refs and views...' }).click();
          const tags = page.getByRole('dialog', { name: 'Guardian statues refs' });
          await tags
            .getByRole('button', { name: 'View of guardian_statues_detail.png: Other' })
            .click();
          await page
            .getByRole('menu', { name: 'View of guardian_statues_detail.png' })
            .getByRole('menuitem', { name: 'Back', exact: true })
            .click();
          await tags
            .getByRole('button', { name: 'View of guardian_statues_detail.png: Back' })
            .waitFor();
          await tags.getByRole('button', { name: 'Done' }).click();
          await page.getByRole('button', { name: 'More for Guardian statues' }).click();
          await page.getByRole('menuitem', { name: 'Send to 3D' }).click();
          const send = page.getByRole('dialog', { name: 'Send Guardian statues to 3D' });
          await send
            .getByRole('group', { name: 'Provider' })
            .getByRole('button', { name: 'Tripo' })
            .click();
          await send.getByText('Multi-view', { exact: true }).click();
          const differ = send.getByText('Back looks different from the front', { exact: false });
          await differ.waitFor();
          await differ.evaluate(el => {
            for (let n = el.parentElement; n; n = n.parentElement) {
              if (
                n.scrollHeight > n.clientHeight + 4 &&
                getComputedStyle(n).overflowY !== 'visible'
              ) {
                n.scrollTop += 112;
                break;
              }
            }
          });
          await page.waitForTimeout(500);
        }
      ),
      delayMs: 1000,
    },
    {
      id: 'plan',
      title: 'Plan',
      caption: 'Plan: a Game mechanics canvas with links, and the cards that overlap flagged.',
      nav: view(
        'scenario=configured&plans=systems&library=demo',
        'plan',
        'Combat systems',
        async page => {
          await page
            .getByRole('region', { name: 'All canvases' })
            .getByRole('button', { name: 'Combat systems', exact: true })
            .click();
          await page.getByRole('main', { name: 'Combat systems canvas' }).waitFor();
          await page.getByRole('button', { name: /^Overlaps/ }).click();
          await page.getByRole('complementary', { name: 'Overlaps' }).waitFor();
          await page.getByRole('button', { name: 'Fit the canvas in view' }).click();
          await page.waitForTimeout(600);
        }
      ),
      delayMs: 1200,
    },
    {
      id: 'camera-plan',
      title: 'Camera plan',
      caption: 'Camera plan: the site tour as stops on a canvas, applied only after a review.',
      nav: view(
        'scenario=configured&plans=demo&library=demo',
        'plan',
        'Camera plan',
        async page => {
          await page.getByRole('region', { name: 'Pinned' }).getByRole('button').click();
          await page.getByRole('main', { name: 'Camera plan canvas' }).waitFor();
          await page
            .getByRole('region', { name: 'Order after Apply' })
            .getByText('Arrival')
            .waitFor();
          // A camera agent proposes a tour with a new stop and a moved one; take it as the main line.
          await page.evaluate(() =>
            window.__TAURI_INTERNALS__.invoke('mock_camera_agent', { project: 'frostveil' })
          );
          await page.getByRole('group', { name: 'Proposal by Camera agent' }).waitFor();
          await page.getByRole('button', { name: 'Use this order' }).click();
          await page
            .getByRole('region', { name: 'Order after Apply' })
            .getByText('new stop')
            .waitFor();
          await page.getByRole('button', { name: 'Fit the canvas in view' }).click();
          // Zoom in so the posters read; the middle stops, the new one and a moved one, stay in view.
          for (let i = 0; i < 3; i++) {
            await page.getByRole('button', { name: 'Zoom in' }).click();
            await page.waitForTimeout(250);
          }
          await page.waitForTimeout(900);
        }
      ),
      delayMs: 1200,
    },
    {
      id: 'recipe-editor',
      title: 'Animation recipes',
      caption: 'Animation recipes: pose keys laid over a clip, with a rough preview of the bake.',
      nav: view('scenario=configured&motion=demo&library=demo', 'motion', 'Recipes', async page => {
        await page.getByRole('tab', { name: 'Recipes' }).click();
        await page
          .getByRole('list', { name: 'Project recipes' })
          .getByRole('listitem')
          .filter({ hasText: 'Slash with a lean' })
          .getByRole('button', { name: 'Edit' })
          .click();
        await page.getByRole('img', { name: /rough preview$/ }).waitFor();
        await page.waitForTimeout(2500);
        // Hold the preview on a chest key, so the pose is the same every run.
        await page.getByRole('button', { name: 'Pause' }).click();
        await page.getByRole('button', { name: 'Pose keys l-1' }).click();
        await page.getByRole('slider', { name: 'Playhead' }).fill('20');
        await page.getByRole('group', { name: 'Change l-1 settings' }).waitFor();
        await page.evaluate(() => document.activeElement?.blur());
        await page.waitForTimeout(900);
      }),
      delayMs: 1500,
    },
    {
      id: 'recipe-review',
      title: 'Recipe review',
      caption: 'Recipe review: the exact bake judged at its seams, with fixes best first.',
      nav: view('scenario=configured&motion=demo&library=demo', 'motion', 'Recipes', async page => {
        await page.getByRole('tab', { name: 'Recipes' }).click();
        const row = page
          .getByRole('list', { name: 'Project recipes' })
          .getByRole('listitem')
          .filter({ hasText: 'Slash into block' });
        await row.getByRole('button', { name: 'Apply' }).click();
        await row.getByRole('button', { name: 'Review' }).waitFor({ timeout: 15000 });
        await row.getByRole('button', { name: 'Review' }).click();
        await page.getByRole('list', { name: 'Review numbers' }).getByText('Seam pop').waitFor();
        const sheet = page.getByRole('region', { name: 'Pose sheet' });
        await sheet.locator('img[src^="data:"]').first().waitFor({ timeout: 20000 });
        await page.waitForTimeout(2500);
        await sheet.getByRole('button', { name: /seam 1/ }).click();
        await page.waitForTimeout(800);
      }),
      delayMs: 1500,
    },
    {
      id: 'built-replace',
      title: 'Built in place',
      caption:
        "Built in place: a script made model takes a generated one's place, settings and all.",
      nav: view('scenario=configured&built=swap', 'board', /^Lantern rebuilt/, async page => {
        await page.getByRole('button', { name: 'More for Lantern rebuilt' }).click();
        await page.getByRole('menuitem', { name: 'Use in place of a model...' }).click();
        const sheet = page.getByRole('dialog', { name: /in place of a model$/ });
        await sheet.waitFor();
        await sheet
          .getByRole('combobox', { name: /Takes the place of/ })
          .selectOption({ label: 'Crystal lantern' });
        await sheet.getByRole('table').waitFor();
        await page.waitForTimeout(800);
      }),
      delayMs: 1200,
    },
    {
      id: 'tool-groups',
      title: 'Tool groups',
      caption: 'Tool groups: switch off the MCP tools an agent never needs.',
      nav: view(
        'scenario=configured&agents=active&library=demo',
        'settings',
        'Agents',
        async page => {
          await page.getByRole('button', { name: 'Agents', exact: true }).click();
          await page
            .getByText('Listening on http://127.0.0.1:4791/mcp', { exact: false })
            .waitFor();
          await page.getByText('Tool groups', { exact: false }).first().waitFor();
          // Settings lays out two columns from 1280 px wide; below that the Agents panel has the
          // whole width, with no empty column beside it.
          await page.setViewportSize({ width: 1200, height: 1000 });
          await page.getByRole('switch', { name: 'Avatar and rig tools' }).uncheck({ force: true });
          await page.getByText('149 of 156 tools offered').waitFor();
          await page
            .getByText('Tool groups', { exact: true })
            .first()
            .evaluate(el => el.scrollIntoView({ block: 'start' }));
          await page.waitForTimeout(600);
        }
      ),
      delayMs: 1200,
    },
    {
      id: 'removed-projects',
      title: 'Removed projects',
      caption:
        'Removed projects: a project moves to the workspace trash and comes back with Restore.',
      nav: view('scenario=configured', 'board', 'Board', async page => {
        // The tool groups shot before this one narrows the window; take the usual size back.
        await page.setViewportSize({ width: 1440, height: 1000 });
        const switcher = page.locator('[aria-haspopup=menu]').first();
        await switcher.click();
        await page.getByRole('menuitem', { name: 'New project' }).click();
        const dialog = page.getByRole('dialog', { name: 'New project' });
        await dialog.waitFor();
        await page.getByLabel('Project name').fill('Lantern study');
        await page.getByRole('button', { name: 'Create project' }).click();
        await dialog.waitFor({ state: 'detached' });
        await switcher.click();
        await page.getByRole('menuitem', { name: 'Remove Lantern study' }).click();
        const ask = page.getByRole('dialog', { name: 'Remove Lantern study?' });
        await ask.waitFor();
        await ask.getByRole('button', { name: 'Remove project' }).click();
        await ask.waitFor({ state: 'detached' });
        await page.getByText('Lantern study is in the workspace trash').waitFor();
        await switcher.click();
        await page.getByRole('menuitem', { name: 'Removed projects' }).click();
        const list = page.getByRole('dialog', { name: 'Removed projects' });
        await list.getByRole('button', { name: 'Restore Lantern study' }).waitFor();
        await page.waitForTimeout(500);
      }),
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
    shots: ['board', 'lab', 'plan'],
    output: 'assets/showcase/hero.webp',
    background: BACKGROUND,
    theme: 'dark',
  },
});
