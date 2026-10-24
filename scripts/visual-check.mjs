/**
 * Renders mock mode in headless Chromium and saves a screenshot of every
 * onboarding step, the Board, the fresh template project, Pipeline (idle,
 * running, failed), the Concept lab (results, a new brief, images
 * generating), the Inbox in every preview mode, the Inspector (compare, a
 * camera stop, budgets), Settings, Settings > Agents with an agent's
 * activity, and the shipping approval prompt. It renders pages to files and never captures the screen.
 *
 *   PLAYWRIGHT=/path/to/playwright/index.mjs OUT_DIR=/tmp/shots pnpm visual
 *
 * PLAYWRIGHT defaults to the `playwright` package, which this repo does not
 * install; point it at any local copy. OUT_DIR defaults to `.visual/`.
 */
import { mkdir } from 'node:fs/promises';
import { isAbsolute, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const root = resolve(import.meta.dirname, '..');
const outDir = resolve(process.env.OUT_DIR ?? resolve(root, '.visual'));
const port = Number(process.env.PORT ?? 15190);
const base = `http://localhost:${port}/`;

/** `import()` needs a file URL for absolute paths on Windows. */
const importable = spec => (isAbsolute(spec) ? pathToFileURL(spec).href : spec);

const { chromium } = await import(importable(process.env.PLAYWRIGHT ?? 'playwright'));

async function serverUp() {
  try {
    return (await fetch(base)).ok;
  } catch {
    return false;
  }
}

/** Reuses a running dev server, else starts one in this process. */
async function startVite() {
  if (await serverUp()) return null;
  const { createServer } = await import(
    importable(resolve(root, 'apps/web/node_modules/vite/dist/node/index.js'))
  );
  const server = await createServer({
    root: resolve(root, 'apps/web'),
    configFile: resolve(root, 'apps/web/vite.config.ts'),
    server: { port, strictPort: true },
    logLevel: 'error',
  });
  await server.listen();
  return server;
}

const vite = await startVite();
await mkdir(outDir, { recursive: true });
const browser = await chromium.launch();
const saved = [];

async function shot(page, name) {
  await page.waitForTimeout(250);
  const path = resolve(outDir, `${name}.png`);
  await page.screenshot({ path });
  saved.push(path);
}

async function open(query = '', hash = '') {
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 1,
  });
  const page = await context.newPage();
  page.on('pageerror', error => console.error(`page error: ${error.message}`));
  await page.goto(`${base}?delay=300${query}${hash}`);
  return page;
}

const cont = page =>
  page.getByRole('button', { name: /^(Continue|Create project|Open .+)$/ }).click();

try {
  const page = await open();
  await page.getByRole('button', { name: 'Set up' }).waitFor();
  await shot(page, 'onboarding-1-welcome');

  await page.getByRole('button', { name: 'Set up' }).click();
  await page.getByText('Empty folder, ready to use').waitFor();
  await shot(page, 'onboarding-2-workspace');

  await cont(page);
  await page.getByText('5 of 6 found').waitFor();
  await shot(page, 'onboarding-3-toolchain');

  await cont(page);
  await page.getByRole('heading', { name: 'Set your defaults' }).waitFor();
  await shot(page, 'onboarding-4-defaults');

  await cont(page);
  await page.getByRole('heading', { name: 'Let your agents drive' }).waitFor();
  await shot(page, 'onboarding-5-agents');

  await cont(page);
  const repo = page.getByLabel('Target site repo');
  await repo.fill('~/Documents/Projects/moon-island-site');
  await repo.press('Enter');
  await page.getByText('Git repo found.').waitFor();
  await shot(page, 'onboarding-6-project');

  await cont(page);
  await page.getByRole('heading', { name: 'Moon Island is ready' }).waitFor();
  await shot(page, 'onboarding-7-ready');

  await cont(page);
  await page.getByRole('heading', { name: 'Your first build in three steps' }).waitFor();
  await shot(page, 'board-empty');

  await page.getByRole('button', { name: 'Pipeline', exact: true }).click();
  await page.getByRole('heading', { name: 'Pipeline runner' }).waitFor();
  await shot(page, 'template-pipeline');

  await page.getByRole('button', { name: 'Settings' }).click();
  await page.getByRole('heading', { name: 'Tripo defaults' }).waitFor();
  await page.getByText('Changes save as you go').waitFor();
  await shot(page, 'settings');

  const blocked = await open('&scenario=no-blender');
  await blocked.getByRole('button', { name: 'Set up' }).click();
  await blocked.getByText('Empty folder, ready to use').waitFor();
  await cont(blocked);
  await blocked.getByText('Blender is required.').waitFor();
  await shot(blocked, 'onboarding-3-toolchain-no-blender');

  const lab = await open('&scenario=configured&conceptMs=60000', '#/lab');
  await lab.getByRole('heading', { name: 'Moon island porch' }).waitFor();
  await shot(lab, 'lab-results');
  await lab.getByRole('button', { name: 'New', exact: true }).click();
  await lab.getByLabel('Direction name').fill('Stone lantern');
  await lab
    .getByLabel('Prompt')
    .fill('A weathered stone lantern on mossy steps, warm candle light, night, soft fog.');
  await lab.getByRole('button', { name: 'Add reference image' }).click();
  await lab.getByRole('button', { name: 'moon-island-concept.png' }).click();
  await shot(lab, 'lab-idle');
  const labRunning = await open('&scenario=configured&lab=running', '#/lab');
  await labRunning.getByText('Image queue').waitFor();
  await shot(labRunning, 'lab-running');

  const agents = await open('&scenario=configured&agents=active', '#/settings');
  await agents.getByRole('heading', { name: 'Tripo defaults' }).waitFor();
  await agents.getByRole('button', { name: 'Agents', exact: true }).click();
  await agents.getByText('Listening on http://127.0.0.1:4791/mcp', { exact: false }).waitFor();
  await agents.waitForTimeout(600);
  await shot(agents, 'settings-agents');
  const prompt = await open('&scenario=configured&agents=active', '#/board');
  await prompt.getByRole('alertdialog').waitFor();
  await shot(prompt, 'agents-approval-prompt');

  const board = await open('&scenario=configured', '#/board');
  await board.getByRole('button', { name: /^Character, v2/ }).waitFor();
  await shot(board, 'board');

  const failed = await open('&scenario=configured', '#/pipeline');
  await failed.getByText('Run 24 failed at Capture posters').first().waitFor();
  await shot(failed, 'pipeline-failed');

  const idle = await open('&scenario=idle', '#/pipeline');
  await idle.getByRole('heading', { name: 'Pipeline runner' }).waitFor();
  await shot(idle, 'pipeline-idle');

  const running = await open('&scenario=running&pace=60', '#/pipeline');
  await running.getByText('Run 25 · step 3 of 6').first().waitFor();
  await running.waitForTimeout(1200);
  await shot(running, 'pipeline-running');

  const inbox = await open('&scenario=configured', '#/inbox');
  await inbox.getByRole('heading', { name: 'Automatic checks' }).waitFor();
  // The preview loads its model lazily; give WebGL a moment.
  await inbox.waitForTimeout(2500);
  await shot(inbox, 'inbox');
  for (const mode of ['Wireframe', 'UV', 'Bones']) {
    await inbox
      .getByRole('group', { name: 'Preview mode' })
      .getByRole('button', { name: mode })
      .click();
    await inbox.waitForTimeout(800);
    await shot(inbox, `inbox-${mode.toLowerCase()}`);
  }
  await inbox.getByRole('button', { name: /Sakura large/ }).click();
  await inbox.waitForTimeout(2500);
  await shot(inbox, 'inbox-prop');
  const inspector = await open('&scenario=configured', '#/inspector');
  await inspector.getByRole('heading', { name: 'Scene comparison' }).waitFor();
  // Both sides load, the camera reaches the first stop and the strip fills in.
  await inspector.getByText(/WebGL 2 · \d+ fps/).waitFor({ timeout: 30_000 });
  await inspector.waitForTimeout(3500);
  await shot(inspector, 'inspector-compare');
  await inspector.getByRole('button', { name: 'Fly to Canopy' }).click();
  await inspector.waitForTimeout(1800);
  await shot(inspector, 'inspector-stop');
  await inspector.getByRole('button', { name: 'Fly to Pullback' }).click();
  await inspector.waitForTimeout(1600);
  await inspector
    .getByRole('group', { name: 'View mode' })
    .getByRole('button', { name: 'Wireframe' })
    .click();
  await inspector.getByRole('button', { name: /^Flower shrubs/ }).click();
  const slider = inspector.getByRole('slider', { name: 'Compare position' });
  for (let i = 0; i < 3; i += 1) await slider.press('ArrowLeft');
  await inspector.mouse.move(700, 880);
  await inspector.waitForTimeout(1200);
  await shot(inspector, 'inspector-budgets');
} finally {
  await browser.close();
  await vite?.close();
}

console.log(saved.join('\n'));
