/**
 * Retakes the app screens the landing page shows (public/scene/screens, and
 * the laptop screen in public/scene) from the web app's mock mode: it starts
 * apps/web's Vite dev server, opens each scenario in headless Chromium with a
 * frozen clock and no network, swaps the sample project's real names for the
 * neutral ones the site has always shown (Frostveil to Winter Spire,
 * Elsheria to Aria) and writes WebP straight over the committed files. It
 * renders pages to files and never captures the screen.
 *
 *   pnpm --filter @maquette/site screens
 *   pnpm --filter @maquette/site screens board,avatar
 *
 * The optional argument is a comma-separated list of ids; with none, every
 * screen is retaken. PLAYWRIGHT defaults to the `playwright` package; PORT
 * picks the dev server port. Run the posters again after the screens change
 * (`pnpm --filter @maquette/site posters`), since the laptop and the monitors
 * in the scene show them.
 */
import { mkdir, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { isAbsolute, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { rename, toWebp } from './screens-page.mjs';

const root = resolve(import.meta.dirname, '..');
const web = resolve(root, '../web');
const sceneDir = resolve(root, 'public/scene');
const screensDir = resolve(sceneDir, 'screens');
/** The laptop screen is the Board at the usual 1440 x 900, scaled down to fit the scene. */
const LAPTOP_SIZE = { width: 1024, height: 640 };
const only = process.argv[2] ? new Set(process.argv[2].split(',')) : null;
const PORT = Number(process.env.PORT ?? 15197);
const ORIGIN = `http://localhost:${PORT}`;
const FROZEN_NOW = new Date('2026-10-23T18:40:00Z');

const importable = spec => (isAbsolute(spec) ? pathToFileURL(spec).href : spec);
const { chromium } = await import(importable(process.env.PLAYWRIGHT ?? 'playwright'));
// The app's own Vite, resolved from apps/web so its config finds its plugins.
const { createServer } = await import(
  pathToFileURL(createRequire(resolve(web, 'package.json')).resolve('vite')).href
);
const server = await createServer({
  root: web,
  configFile: resolve(web, 'vite.config.ts'),
  server: { port: PORT, strictPort: true },
  logLevel: 'error',
});
await server.listen();
await mkdir(screensDir, { recursive: true });
const browser = await chromium.launch({
  args: ['--use-angle=metal', '--enable-gpu', '--ignore-gpu-blocklist'],
});

async function open(query, hash, viewport = { width: 1440, height: 900 }) {
  const context = await browser.newContext({
    viewport,
    deviceScaleFactor: 1,
    colorScheme: 'dark',
    timezoneId: 'Europe/Warsaw',
    reducedMotion: 'reduce',
  });
  context.setDefaultTimeout(90000);
  await context.route(
    url => !url.href.startsWith(ORIGIN) && !url.href.startsWith('data:'),
    route => route.abort()
  );
  const page = await context.newPage();
  await page.clock.setFixedTime(FROZEN_NOW);
  page.on('pageerror', e => console.error(`page error: ${e.message}`));
  await page.goto(`${ORIGIN}/?delay=300${query ? `&${query}` : ''}${hash ? `#/${hash}` : ''}`);
  return page;
}

async function save(page, name, target = page, clip) {
  await page.mouse.move(page.viewportSize().width - 1, page.viewportSize().height - 1);
  await page.waitForTimeout(400);
  await page.evaluate(rename);
  await page.waitForTimeout(100);
  const png = clip
    ? await page.screenshot({ clip })
    : await target.screenshot({ animations: 'disabled' });
  const laptop = name === 'laptop-screen';
  const file = resolve(laptop ? sceneDir : screensDir, `${name}.webp`);
  await writeFile(file, await toWebp(page, png, laptop ? LAPTOP_SIZE : undefined));
  console.log(`ok ${name}`);
  await page.context().close();
}

const union = async (...locs) => {
  const boxes = await Promise.all(locs.map(l => l.boundingBox()));
  const x = Math.min(...boxes.map(b => b.x));
  const y = Math.min(...boxes.map(b => b.y));
  const r = Math.max(...boxes.map(b => b.x + b.width));
  const btm = Math.max(...boxes.map(b => b.y + b.height));
  return { x, y, width: r - x, height: btm - y };
};

async function runningPipeline() {
  const p = await open('scenario=running&pace=60&library=demo', 'pipeline');
  await p.getByText('Run 25 · step 3 of 6').first().waitFor();
  await p.getByRole('button', { name: 'Keep mine' }).click();
  await p.waitForTimeout(1500);
  return p;
}

/** Opens the Guardian statues card menu and picks an item. */
async function cardMenu(p, item) {
  await p.getByRole('button', { name: 'More for Guardian statues' }).click();
  await p.getByRole('menuitem', { name: item }).click();
}

/** Tags the detail ref as the back view in the refs dialog and returns it. */
async function tagBackView(p) {
  await p.getByText('Guardian statues').first().waitFor();
  await cardMenu(p, 'Refs and views...');
  const tags = p.getByRole('dialog', { name: 'Guardian statues refs' });
  const view = 'View of guardian_statues_detail.png';
  await tags.getByRole('button', { name: `${view}: Other` }).click();
  await p
    .getByRole('menu', { name: view })
    .getByRole('menuitem', { name: 'Back', exact: true })
    .click();
  await tags.getByRole('button', { name: `${view}: Back` }).waitFor();
  return tags;
}

const REFS = 'scenario=configured&lab=refs&library=demo';
const TALL = { width: 1440, height: 1000 };
async function boardShot(name, viewport) {
  const p = await open(IDLE, 'board', viewport);
  await p.getByText('Update to v3').first().waitFor();
  await p.waitForTimeout(1200);
  await save(p, name);
}

async function inspector(settle) {
  const p = await open(IDLE, 'inspector');
  await p
    .getByText(/WebGL 2 · \d+ fps/)
    .first()
    .waitFor();
  await p.waitForTimeout(settle);
  return p;
}

const IDLE = 'scenario=idle&library=demo';

const shots = {
  board: () => boardShot('board'),
  'laptop-screen': () => boardShot('laptop-screen'),
  async lab() {
    const p = await open(IDLE, 'lab');
    await p.getByText('Frozen lake arena').first().waitFor();
    await p.waitForTimeout(1200);
    await save(p, 'lab');
  },
  async inbox() {
    const p = await open(IDLE, 'inbox');
    await p.getByText('Automatic checks').first().waitFor();
    await p.waitForTimeout(3000);
    await save(p, 'inbox');
  },
  async 'pipeline-running'() {
    const p = await runningPipeline();
    await save(p, 'pipeline-running');
  },
  async 'pipeline-compress'() {
    const p = await runningPipeline();
    const card = "xpath=ancestor::*[contains(concat(' ', @class, ' '), ' rounded-card ')][1]";
    const logs = p.getByRole('tab', { name: 'Logs' }).locator(card);
    const presets = p.getByText('Compression presets', { exact: true }).first().locator(card);
    await save(p, 'pipeline-compress', p, await union(logs, presets));
  },
  async 'pipeline-done'() {
    const p = await open(IDLE, 'pipeline');
    await p.getByText('Pipeline runner').first().waitFor();
    await p.getByRole('button', { name: 'Keep mine' }).click();
    await p.waitForTimeout(1500);
    await save(p, 'pipeline-done');
  },
  async 'inspector-compare'() {
    const p = await inspector(3500);
    await save(p, 'inspector-compare');
  },
  async 'inspector-budgets'() {
    const p = await inspector(2500);
    const budgets = p.getByText('Model budgets', { exact: true });
    await budgets.waitFor();
    const total = p.getByText('Scene total', { exact: true });
    const lib = p.getByText('Library', { exact: true }).last();
    const top = (await lib.count()) ? lib : p.getByText('Textures', { exact: true });
    await top.evaluate(el => el.scrollIntoView({ block: 'start' }));
    await p.waitForTimeout(500);
    const a = await top.boundingBox();
    const b = await total.boundingBox();
    const panel = await budgets.locator('xpath=ancestor::aside[1]').boundingBox();
    const x = panel ? panel.x : a.x - 20;
    const width = panel ? panel.width : 340;
    await save(p, 'inspector-budgets', p, {
      x,
      y: a.y - 20,
      width,
      height: b.y + b.height + 20 - (a.y - 20),
    });
  },
  async parts() {
    const p = await open('scenario=configured&parts=demo&library=demo', 'inspector');
    await p.getByRole('tab', { name: 'Parts' }).click();
    const list = p.getByRole('list', { name: 'Parts' });
    await list.waitFor();
    await list.getByRole('button', { name: 'cage' }).click();
    await list.getByRole('button', { name: 'crystal' }).click({ modifiers: ['Shift'] });
    await p.getByText('cage and crystal selected').waitFor();
    await p.waitForTimeout(2500);
    await save(p, 'parts');
  },
  async animations() {
    const p = await open('scenario=configured&library=demo', 'animations');
    await p
      .getByRole('complementary', { name: 'Selected clip' })
      .getByRole('heading', { name: 'Standing Dodge Left' })
      .waitFor();
    await p.waitForTimeout(2500);
    await save(p, 'animations');
  },
  async retarget() {
    const p = await open('scenario=configured&library=demo&motion=demo', 'motion');
    await p.getByText('119 bones · 2 clips').first().waitFor();
    await p.getByRole('button', { name: 'Add clip' }).click();
    await p.getByRole('menuitem', { name: 'Add from library' }).click();
    const steps = p.getByRole('dialog', { name: /^Add library clips to / });
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
    const review = p.getByRole('dialog', { name: /^Review the retarget on / });
    await review.waitFor();
    await p.waitForTimeout(2500);
    await p.getByRole('button', { name: 'Pin the feet and bake again' }).click();
    await p.getByRole('group', { name: 'Feet pinned' }).waitFor();
    await p.waitForTimeout(2500);
    await save(p, 'retarget', review);
  },
  async 'agents-approval'() {
    const p = await open('scenario=idle&agents=active&library=demo', 'board');
    await p.getByText('Send Rune obelisk to Tripo?').first().waitFor();
    await p.waitForTimeout(1200);
    await save(p, 'agents-approval');
  },
  async 'settings-agents'() {
    const p = await open('scenario=configured&agents=active&library=demo', 'settings');
    await p.getByRole('heading', { name: '3D providers' }).first().waitFor();
    await p.getByRole('button', { name: 'Agents', exact: true }).click();
    await p.getByText('Listening on http://127.0.0.1:4791/mcp', { exact: false }).waitFor();
    await p.waitForTimeout(800);
    await save(p, 'settings-agents');
  },
  async 'settings-providers'() {
    const p = await open('scenario=configured&library=demo', 'settings');
    await p.getByRole('heading', { name: '3D providers' }).first().waitFor();
    await p.getByRole('button', { name: '3D providers', exact: true }).click();
    await p.waitForTimeout(800);
    await save(p, 'settings-providers');
  },
  async toolchain() {
    const p = await open('', '');
    await p.getByRole('button', { name: 'Set up' }).click();
    await p.getByText('Empty folder, ready to use').waitFor();
    await p.getByRole('button', { name: /^Continue$/ }).click();
    await p.getByText('5 of 6 found').waitFor();
    await p.waitForTimeout(600);
    await save(p, 'toolchain');
  },
  async 'send-tripo'() {
    const p = await open('scenario=configured', 'board');
    await cardMenu(p, 'Send to 3D');
    const d = p.getByRole('dialog', { name: /Send Guardian statues to 3D/ });
    await d.waitFor();
    await d.getByText('1 of 4').waitFor();
    await p.waitForTimeout(600);
    await save(p, 'send-tripo', d);
  },
  async 'by-hand'() {
    const p = await open('scenario=configured', 'board');
    await cardMenu(p, 'Send to 3D');
    const d = p.getByRole('dialog', { name: /Send Guardian statues to 3D/ });
    await d.waitFor();
    await d
      .getByRole('group', { name: 'Provider' })
      .getByRole('button', { name: 'By hand' })
      .click();
    await d
      .getByRole('group', { name: 'Settings card for' })
      .getByRole('button', { name: 'Tripo' })
      .click();
    await d.getByRole('button', { name: 'Write the handoff folder' }).click();
    await p.getByText('Bring the GLB back', { exact: true }).waitFor();
    await p.waitForTimeout(600);
    const card = p.getByRole('dialog').last();
    await save(p, 'by-hand', card);
  },
  async 'refs-views'() {
    const p = await open(REFS, 'board', TALL);
    const tags = await tagBackView(p);
    await p.waitForTimeout(800);
    await save(p, 'refs-views', tags);
  },
  async 'ref-warnings'() {
    const p = await open(REFS, 'board', TALL);
    const tags = await tagBackView(p);
    await tags.getByRole('button', { name: 'Done' }).click();
    await cardMenu(p, 'Send to 3D');
    const send = p.getByRole('dialog', { name: 'Send Guardian statues to 3D' });
    await send
      .getByRole('group', { name: 'Provider' })
      .getByRole('button', { name: 'Tripo' })
      .click();
    await send.getByText('Multi-view', { exact: true }).click();
    const differ = send.getByText('Back looks different from the front', { exact: false });
    await differ.waitFor();
    await differ.evaluate(el => {
      for (let n = el.parentElement; n; n = n.parentElement) {
        if (n.scrollHeight > n.clientHeight + 4 && getComputedStyle(n).overflowY !== 'visible') {
          n.scrollTop += 112;
          break;
        }
      }
    });
    await p.waitForTimeout(600);
    await save(p, 'ref-warnings', send);
  },
  async avatar() {
    const p = await open('scenario=configured&avatar=pending&library=demo', 'avatar');
    const approval = p.getByRole('region', { name: 'Rest pose approval' });
    await approval.waitFor();
    await p.getByRole('img', { name: 'The after sheet' }).last().waitFor();
    await approval.evaluate(el => el.scrollIntoView({ block: 'start' }));
    await p.waitForTimeout(1200);
    await save(p, 'avatar');
  },
};

const unknown = [...(only ?? [])].filter(id => !(id in shots));
if (unknown.length) {
  console.error(`unknown ids: ${unknown.join(', ')}\nknown: ${Object.keys(shots).join(', ')}`);
  process.exit(2);
}

let failed = 0;
for (const [name, fn] of Object.entries(shots)) {
  if (only && !only.has(name)) continue;
  try {
    await fn();
  } catch (e) {
    failed++;
    console.error(`FAIL ${name}: ${e.message.split('\n')[0]}`);
  }
}
await browser.close();
await server.close();
process.exit(failed ? 1 : 0);
