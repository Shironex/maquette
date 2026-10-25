/**
 * Renders mock mode in headless Chromium and saves a screenshot of every
 * onboarding step, the Board, the fresh template project, Pipeline (idle,
 * running, failed), the Concept lab (results, a new brief, images
 * generating), the Inbox in every preview mode, the Inspector (compare, a
 * camera stop, budgets, Export GLB), Settings, Settings > Agents with an
 * agent's activity, the approval prompts (Ship and a send to 3D), Send to 3D
 * for Tripo, Meshy and by hand, the by hand settings card, the 3D queue, a
 * Board card waiting for a model, Send to 3D from the Lab and Settings > 3D
 * providers with a tested key, the project switcher (open, with search, the
 * New project dialog, the collapsed rail, the Board after a switch), moving
 * a Lab direction, a long Lab title, a Refs card with its picture and the
 * starter scripts banner. The handoff set covers Make object refs, an
 * objects direction, Board multi-select, the batch send panel (By hand and
 * Tripo), the By hand checklist, Add model and Check Downloads, and a batch
 * approval from an agent. The scene set covers the Inspector's layout editor
 * (a selected model, a camera stop, looking through it), Copy to project on
 * a Board card, the first view in the Pipeline view and the first view
 * target in Settings. The ux set covers tooltips (the collapsed rail, a sheet's
 * Close by the window edge, a gizmo key, a disabled Undo), help marks, a
 * keyboard focus ring and a disabled menu item that says why. The material
 * set covers the Inspector's Material panel (untouched, an edit previewing
 * on the model, one material by name, applied), the per map codec rows and
 * the quit question for tweaks. The redo set covers Redo from the Inbox (a
 * row's menu, the send panel prefilled with a Meshy texture prompt and its
 * estimate, two models ticked, the batch redo and its confirm) and the
 * Credits sheet (all sends, filtered to Meshy). It renders pages to files
 * and never captures the screen.
 *
 *   PLAYWRIGHT=/path/to/playwright/index.mjs OUT_DIR=/tmp/shots pnpm visual
 *
 * ONLY=handoff, ONLY=scene, ONLY=ux, ONLY=material or ONLY=redo renders that
 * set alone, ONLY=guards the questions asked before work is dropped; PORT
 * picks the dev server port.
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

/** Shoots a page, or one element when given its locator. */
async function shot(target, name) {
  await (target.page?.() ?? target).waitForTimeout(250);
  const path = resolve(outDir, `${name}.png`);
  await target.screenshot({ path });
  saved.push(path);
}

async function open(query = '', hash = '', viewport = { width: 1440, height: 900 }) {
  const context = await browser.newContext({
    viewport,
    deviceScaleFactor: 1,
  });
  const page = await context.newPage();
  page.on('pageerror', error => console.error(`page error: ${error.message}`));
  await page.goto(`${base}?delay=300${query}${hash}`);
  return page;
}

const cont = page =>
  page.getByRole('button', { name: /^(Continue|Create project|Open .+)$/ }).click();

/** Object refs, batch sends and the By hand hand-back, on the workbench fixture. */
async function handoffShots() {
  const fixture = '&scenario=configured&handoff=batch&conceptMs=60000';
  const lab = await open(fixture, '#/lab');
  await lab.getByText('Earlier directions').waitFor();
  await lab.getByRole('button', { name: /^Workbench hero 1 image/ }).click();
  await lab.getByRole('heading', { name: 'Workbench hero', exact: true }).waitFor();
  await lab.getByRole('button', { name: 'Make object refs from Concept 01' }).click();
  const field = lab.getByRole('textbox', { name: 'Objects' });
  await field.fill('desk lamp, sculptor figure, clay island, turntable island, brush cup');
  await field.press('Enter');
  await field.fill('calipers');
  await lab.getByRole('dialog', { name: 'Make object refs' }).waitFor();
  await shot(lab, 'objects-dialog');

  // A long name stays whole with a counter; a chip opens its name and notes.
  await field.fill(
    'Finished island in painted stone and moss with no wireframe lines and no rocks'
  );
  await shot(lab, 'objects-long-name');
  await field.fill('');
  await lab.getByRole('button', { name: 'Edit Clay island' }).click();
  const editor = lab.getByRole('group', { name: 'Edit Clay island' });
  await editor
    .getByRole('textbox', { name: 'Notes for Codex' })
    .fill('Painted stone and moss, no wireframe lines, no loose floating rocks');
  await shot(lab, 'objects-notes');
  await editor.getByRole('button', { name: 'Done' }).click();
  await shot(lab, 'objects-note-chip');

  const porch = await open('&scenario=configured&conceptMs=60000', '#/lab');
  await porch.getByRole('heading', { name: 'Moon island porch' }).waitFor();
  await porch.getByRole('button', { name: 'Make object refs from Concept 01' }).click();
  const names = porch.getByRole('textbox', { name: 'Objects' });
  await names.fill('bench, vending machine, paper lantern, box sets');
  await names.press('Enter');
  await porch.getByRole('button', { name: 'Make 4 object refs' }).click();
  await porch.getByRole('heading', { name: 'Moon island porch objects' }).waitFor();
  await shot(porch, 'objects-generating');

  const objects = await open(fixture, '#/lab');
  await objects.getByText('Earlier directions').waitFor();
  await objects.getByRole('button', { name: /^Workbench hero objects/ }).click();
  await objects.getByRole('heading', { name: 'Workbench hero objects' }).waitFor();
  await objects.waitForTimeout(400);
  await shot(objects, 'objects-direction');
  await objects.getByRole('button', { name: 'Send to 3D' }).click();
  await objects.getByRole('dialog', { name: 'Send 6 assets to 3D' }).waitFor();
  await objects.waitForTimeout(300);
  await shot(objects, 'batch-by-hand');

  const board = await open(fixture, '#/board');
  await board.getByRole('button', { name: /^Character, v2/ }).waitFor();
  await board.getByRole('button', { name: 'More for Desk lamp' }).click();
  await board.getByRole('menuitem', { name: 'Rename...' }).waitFor();
  await shot(board, 'asset-rename-menu');
  await board.getByRole('menuitem', { name: 'Rename...' }).click();
  await board.getByRole('dialog', { name: 'Rename asset' }).waitFor();
  await shot(board, 'asset-rename-dialog');
  await board.keyboard.press('Escape');
  await board.getByRole('checkbox', { name: 'Select Turntable island' }).click();
  await board
    .getByRole('checkbox', { name: 'Select Vending machine' })
    .click({ modifiers: ['Shift'] });
  await board.getByRole('checkbox', { name: 'Select Island' }).click();
  const selection = board.getByRole('toolbar', { name: 'Selected assets' });
  await selection.waitFor();
  await board.mouse.move(0, 0);
  await shot(board, 'board-multi-select');
  await selection.getByRole('button', { name: 'Clear' }).click();
  await board.getByRole('checkbox', { name: 'Select Sakura' }).click();
  await board
    .getByRole('checkbox', { name: 'Select Stone lanterns' })
    .click({ modifiers: ['Shift'] });
  for (const name of ['Sculptor figure', 'Desk lamp'])
    await board.getByRole('checkbox', { name: `Select ${name}` }).click();
  await board
    .getByRole('toolbar', { name: 'Selected assets' })
    .getByRole('button', { name: 'Send to 3D' })
    .click();
  const batch = board.getByRole('dialog', { name: /^Send \d+ assets to 3D$/ });
  await batch.waitFor();
  await batch
    .getByRole('group', { name: 'Provider' })
    .getByRole('button', { name: 'Tripo' })
    .click();
  await board.waitForTimeout(300);
  await shot(board, 'batch-tripo');
  await batch.getByRole('button', { name: 'Custom settings for Desk lamp' }).click();
  const own = batch.getByRole('group', { name: 'Desk lamp settings' });
  const faces = own.getByRole('textbox', { name: 'Face limit' });
  await faces.fill('12000');
  await faces.press('Enter');
  await own.getByText('PBR', { exact: true }).click();
  await batch.getByRole('button', { name: 'Custom settings for Sculptor figure' }).click();
  const sculptor = batch.getByRole('group', { name: 'Sculptor figure settings' });
  const sculptFaces = sculptor.getByRole('textbox', { name: 'Face limit' });
  await sculptFaces.fill('2000');
  await sculptFaces.press('Enter');
  await board.waitForTimeout(300);
  await shot(board, 'batch-custom-settings');
  await sculptor.getByRole('button', { name: 'Use batch settings' }).click();
  await batch.getByRole('button', { name: 'Custom settings for Sculptor figure' }).click();
  await batch.getByRole('button', { name: /^Send \d+ to Tripo$/ }).click();
  await batch.getByRole('alert').waitFor();
  await shot(board, 'batch-tripo-confirm');
  // The batch has unsent edits, so Escape asks before it drops them.
  await board.keyboard.press('Escape');
  const discard = board.getByRole('dialog', { name: 'Discard changes?' });
  await discard.getByRole('button', { name: 'Discard' }).click();
  await batch.waitFor({ state: 'detached' });

  // The Board's By hand button carries its count; the panel's provider choice is gone.
  await board.getByRole('button', { name: /^By hand \d+$/ }).click();
  await board.getByRole('dialog', { name: 'By hand checklist' }).waitFor();
  await shot(board, 'handoff-checklist');
  await board.getByRole('button', { name: 'Settings card' }).first().click();
  await board.getByText('Bring the GLB back', { exact: true }).waitFor();
  await shot(board, 'handoff-checklist-card');
  await board
    .getByRole('dialog', { name: 'By hand checklist' })
    .getByRole('button', { name: 'Check Downloads' })
    .first()
    .click();
  await board
    .getByRole('dialog', { name: 'New models in Downloads' })
    .getByText('model (3).glb')
    .waitFor();
  await shot(board, 'downloads-matches');

  const card = await open(fixture, '#/board');
  await card.getByRole('button', { name: /^Character, v2/ }).waitFor();
  await card.getByRole('button', { name: 'More for Brush cup' }).click();
  await card.getByRole('menu', { name: 'Brush cup actions' }).waitFor();
  await shot(card, 'board-add-model');
  await card.keyboard.press('Escape');
  await card.getByRole('button', { name: 'Add model...' }).first().click();
  await card.getByText('is in the Inbox', { exact: false }).first().waitFor();
  await shot(card, 'board-model-added');

  const approval = await open('&scenario=configured&agents=batch', '#/board');
  await approval.getByRole('alertdialog', { name: 'Send 3 assets to Tripo?' }).waitFor();
  await shot(approval, 'agents-batch-approval');
}

/** The questions asked before work is dropped: Send to 3D, Settings and quit. */
async function guardShots() {
  const board = await open('&scenario=configured', '#/board');
  await board.getByRole('button', { name: 'More for Stone lanterns' }).click();
  await board.getByRole('menuitem', { name: 'Send to 3D' }).click();
  await board.getByRole('dialog', { name: /Send Stone lanterns to 3D/ }).waitFor();
  await board.getByRole('switch', { name: 'PBR' }).click({ force: true });
  await board.keyboard.press('Escape');
  await board.getByRole('dialog', { name: 'Discard changes?' }).waitFor();
  await shot(board, 'guard-send-discard');

  const settings = await open('&scenario=configured', '#/settings');
  const faces = settings.getByRole('textbox', { name: 'Face limit' }).first();
  await faces.fill('30000');
  await faces.press('Enter');
  await settings.getByRole('combobox', { name: 'Model' }).first().selectOption('P1-20260311');
  await settings.getByRole('button', { name: 'Board' }).first().click();
  await settings.getByRole('dialog', { name: 'Some settings are not valid' }).waitFor();
  await shot(settings, 'guard-settings-invalid');

  const quit = await open('&scenario=configured', '#/pipeline');
  await quit.getByRole('heading', { name: 'Compression presets' }).waitFor();
  await quit.evaluate(async () => {
    // The dev server shares its modules with the page, so the stores are reachable.
    const { usePipelineStore } = await import('/src/state/use-pipeline-store.ts');
    const { useQuitStore } = await import('/src/state/use-quit-store.ts');
    const view = usePipelineStore.getState().view;
    const model = view.models[0].model;
    usePipelineStore.getState().setPresetEdits(view.project, {
      [model]: {
        model,
        label: model,
        textureSize: 512,
        dataTextureSize: null,
        codec: 'etc1s',
        mapCodecs: null,
        meshopt: 'low',
        lock: null,
        custom: true,
      },
    });
    useQuitStore.getState().ask();
  });
  await quit.getByRole('dialog', { name: 'Apply the compression presets?' }).waitFor();
  await shot(quit, 'guard-quit');
}

const only = process.env.ONLY;

/** Hovers a control until its tooltip shows, then shoots the page. */
async function hoverTip(page, target, name) {
  await target.hover();
  await page.getByRole('tooltip').waitFor();
  await shot(page, name);
  await page.mouse.move(0, 0);
}

/** Focuses a control from the keyboard (its ring and tooltip show), then shoots. */
async function focusTip(page, target, name) {
  await target.focus();
  await page.keyboard.press('Shift+Tab');
  await page.keyboard.press('Tab');
  await page.getByRole('tooltip').waitFor();
  await shot(page, name);
}

/** Tooltips, help marks and the disabled with a reason state. */
async function uxShots() {
  const board = await open('&scenario=configured', '#/board');
  await board.getByRole('button', { name: /^Character, v2/ }).waitFor();
  await hoverTip(board, board.getByRole('checkbox', { name: 'Select Island' }), 'ux-card-select');
  await board.getByRole('button', { name: 'More for Stone lanterns' }).click();
  await hoverTip(
    board,
    board.getByRole('menuitem', { name: 'Open in Blender' }),
    'ux-menu-disabled-reason'
  );
  await board.keyboard.press('Escape');
  await board.getByRole('button', { name: 'More for Stone lanterns' }).click();
  await board.getByRole('menuitem', { name: 'Send to 3D' }).click();
  const sheet = board.getByRole('dialog', { name: /Send Stone lanterns to 3D/ });
  await sheet.waitFor();
  await hoverTip(board, sheet.getByRole('button', { name: 'Close' }), 'ux-sheet-close-edge');

  const rail = await open('&scenario=configured', '#/board');
  await rail.getByRole('button', { name: 'Collapse' }).click();
  await hoverTip(rail, rail.getByRole('button', { name: /^Inbox/ }), 'ux-rail-collapsed');

  const fresh = await open('&scenario=running', '#/board');
  await fresh.locator('[aria-haspopup=menu]').first().click();
  await fresh.getByRole('menuitem', { name: 'New project' }).click();
  const create = fresh.getByRole('button', { name: 'Create project' });
  await create.waitFor();
  await hoverTip(fresh, create, 'ux-disabled-reason');

  const logs = await open('&scenario=configured', '#/pipeline');
  await logs.getByRole('log').getByText('Traceback').waitFor();
  await focusTip(logs, logs.getByRole('button', { name: 'About hiding noise' }), 'ux-help-noise');
  await hoverTip(
    logs,
    logs.getByRole('button', { name: /^Texture sizes per map for/ }).first(),
    'ux-preset-per-map'
  );

  const layout = await open('&scenario=configured', '#/inspector');
  await layout.getByRole('tab', { name: 'Layout' }).click();
  await layout.getByRole('heading', { name: 'Scene layout' }).waitFor();
  await layout.getByText(/Loading \d+ models/).waitFor({ state: 'detached', timeout: 30_000 });
  await layout.waitForTimeout(1500);
  await hoverTip(layout, layout.getByRole('button', { name: 'Undo' }), 'ux-layout-undo-disabled');
  const gizmo = layout.getByRole('group', { name: 'Gizmo' });
  await hoverTip(layout, gizmo.getByRole('button', { name: 'Turn' }), 'ux-layout-gizmo-key');
  await focusTip(
    layout,
    layout.getByRole('button', { name: 'Frame every model' }),
    'ux-focus-ring'
  );
  await focusTip(
    layout,
    layout.getByRole('button', { name: 'About the layout keys' }),
    'ux-help-layout-keys'
  );
}

/** The layout editor, Copy to project and the first view. */
async function sceneShots() {
  const layout = await open('&scenario=configured', '#/inspector');
  await layout.getByRole('tab', { name: 'Layout' }).click();
  await layout.getByRole('heading', { name: 'Scene layout' }).waitFor();
  await layout.getByText(/Loading \d+ models/).waitFor({ state: 'detached', timeout: 30_000 });
  await layout.waitForTimeout(2500);
  await shot(layout, 'scene-layout');
  const details = layout.getByRole('complementary', { name: 'Layout details' });
  await details.getByRole('button', { name: /^Character/ }).click();
  await layout.waitForTimeout(800);
  await shot(layout, 'scene-layout-model');
  await layout.keyboard.press('e');
  await layout.waitForTimeout(500);
  await shot(layout, 'scene-layout-rotate');
  await details.getByRole('button', { name: /^1 Porch/ }).click();
  await layout.waitForTimeout(600);
  await shot(layout, 'scene-layout-stop');
  await details.getByRole('button', { name: 'Look through', exact: true }).click();
  await layout.waitForTimeout(1800);
  await shot(layout, 'scene-layout-look');

  const board = await open('&scenario=configured', '#/board');
  await board.getByRole('button', { name: 'More for Character' }).click();
  await board.getByRole('menuitem', { name: 'Copy to project...' }).click();
  await board.getByRole('menu', { name: 'Copy Character to project' }).waitFor();
  await shot(board, 'board-copy-to-project');
  await board.getByRole('menuitem', { name: 'Rooftop cinema' }).click();
  await board.getByText('Copied Character to Rooftop cinema').waitFor();
  await shot(board, 'board-copied');

  const pipeline = await open('&scenario=idle', '#/pipeline');
  await pipeline.getByText('First view · Porch').waitFor();
  await shot(pipeline, 'pipeline-first-view');

  const settings = await open('&scenario=configured', '#/settings');
  await settings.getByLabel('First view target').waitFor();
  await settings.getByLabel('First view target').scrollIntoViewIfNeeded();
  await shot(settings, 'settings-first-view');
}

/** The Material panel and its live preview, the per map codecs and the quit question. */
async function materialShots() {
  const page = await open('&scenario=configured', '#/inspector', { width: 1440, height: 1500 });
  await page.getByRole('heading', { name: 'Scene comparison' }).waitFor();
  await page.getByText(/WebGL 2 · \d+ fps/).waitFor({ timeout: 30_000 });
  await page.waitForTimeout(3500);
  const details = page.getByRole('complementary', { name: 'Model details' });
  await details.getByRole('button', { name: /^Character/ }).click();
  await details.getByRole('heading', { name: 'Material' }).waitFor();
  await page.waitForTimeout(600);
  await shot(page, 'material-before');
  await shot(details, 'material-panel');

  const hex = details.getByRole('textbox', { name: 'Tint hex' });
  await hex.fill('#5b7fd6');
  await hex.press('Enter');
  const set = (group, name) =>
    details.getByRole('group', { name: group }).getByRole('button', { name, exact: true });
  await set('Roughness', 'Set').click();
  const roughness = details.getByRole('textbox', { name: 'Roughness value' });
  await roughness.fill('0.25');
  await roughness.press('Enter');
  await set('Metalness', 'Set').click();
  const metalness = details.getByRole('textbox', { name: 'Metalness value' });
  await metalness.fill('0.6');
  await metalness.press('Enter');
  await details.getByText('Not applied yet').waitFor();
  await page.waitForTimeout(800);
  await shot(page, 'material-preview');
  await shot(details, 'material-panel-edited');

  await details
    .getByRole('combobox', { name: 'Applies to' })
    .selectOption('tripo_mat_90739555.001');
  await set('Emissive', 'Set').click();
  await page.waitForTimeout(400);
  await shot(details, 'material-panel-named');

  await page.evaluate(async () => {
    const { useQuitStore } = await import('/src/state/use-quit-store.ts');
    useQuitStore.getState().ask();
  });
  await page.getByRole('dialog', { name: 'Apply the material tweaks?' }).waitFor();
  await shot(page, 'material-quit');
  await page.getByRole('button', { name: 'Cancel' }).click();

  await details.getByRole('button', { name: 'Apply tweaks' }).click();
  await page.getByText('Material tweaks saved').waitFor();
  await shot(page, 'material-applied');
}

/** Every other view, from onboarding to the starter scripts banner. */
async function coreShots() {
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
  await page.getByRole('heading', { name: '3D providers' }).first().waitFor();
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
  await agents.getByRole('heading', { name: '3D providers' }).first().waitFor();
  await agents.getByRole('button', { name: 'Agents', exact: true }).click();
  await agents.getByText('Listening on http://127.0.0.1:4791/mcp', { exact: false }).waitFor();
  await agents.waitForTimeout(600);
  await shot(agents, 'settings-agents');
  const prompt = await open('&scenario=configured&agents=active', '#/board');
  await prompt.getByRole('alertdialog', { name: 'Send Far spire to Tripo?' }).waitFor();
  await shot(prompt, 'agents-approval-prompt');

  const board = await open('&scenario=configured', '#/board');
  await board.getByRole('button', { name: /^Character, v2/ }).waitFor();
  await shot(board, 'board');
  await board.getByRole('button', { name: /3D queue/ }).click();
  await board.getByRole('region', { name: '3D queue' }).waitFor();
  await shot(board, 'provider-queue');
  await board.keyboard.press('Escape');
  await board.getByRole('button', { name: 'More for Stone lanterns' }).click();
  await board.getByRole('menuitem', { name: 'Send to 3D' }).click();
  await board.getByRole('dialog', { name: /Send Stone lanterns to 3D/ }).waitFor();
  await board.getByText('1 of 4').waitFor();
  await shot(board, 'send-tripo');
  const providers = board.getByRole('group', { name: 'Provider' });
  await providers.getByRole('button', { name: 'Meshy' }).click();
  await shot(board, 'send-meshy');
  await providers.getByRole('button', { name: 'By hand' }).click();
  await board
    .getByRole('group', { name: 'Settings card for' })
    .getByRole('button', { name: 'Tripo' })
    .click();
  await shot(board, 'send-by-hand');
  await board.getByRole('button', { name: 'Write the handoff folder' }).click();
  await board.getByText('Bring the GLB back', { exact: true }).waitFor();
  await shot(board, 'by-hand-card');

  const labSend = await open('&scenario=configured', '#/lab');
  await labSend.getByRole('button', { name: 'Send to 3D' }).click();
  await labSend.getByRole('menuitem', { name: 'As a scene' }).click();
  await labSend.getByRole('dialog', { name: /to 3D/ }).waitFor();
  await labSend.waitForTimeout(400);
  await shot(labSend, 'lab-send-to-3d');

  const keys = await open('&scenario=configured', '#/settings');
  await keys.getByRole('button', { name: '3D providers', exact: true }).click();
  await keys.getByRole('button', { name: 'Test key' }).click();
  await keys.getByText('The key works.', { exact: false }).waitFor();
  await shot(keys, 'settings-providers');

  const failed = await open('&scenario=configured', '#/pipeline');
  await failed.getByText('Run 24 failed at Capture posters').first().waitFor();
  await shot(failed, 'pipeline-failed');
  // A tall window, so the whole log shows.
  const logs = await open('&scenario=configured', '#/pipeline', { width: 1440, height: 1500 });
  await logs.getByRole('log').getByText('Traceback').waitFor();
  await shot(logs.getByRole('log'), 'pipeline-log');
  await logs.getByText('Hide noise').click();
  await shot(logs.getByRole('log'), 'pipeline-log-quiet');

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

  // The project switcher.
  const switcher = await open('&scenario=running', '#/board');
  const projectButton = switcher.locator('[aria-haspopup=menu]').first();
  await projectButton.waitFor();
  await shot(switcher, 'board-refs-card');
  await switcher.keyboard.press(process.platform === 'darwin' ? 'Meta+p' : 'Control+p');
  await switcher.getByRole('menu', { name: 'Projects' }).waitFor();
  await switcher.getByText(/Character showcase · 1 asset/).waitFor();
  await shot(switcher, 'switcher-open');
  await switcher.getByRole('menuitem', { name: 'New project' }).click();
  await switcher.getByRole('dialog', { name: 'New project' }).waitFor();
  await switcher.getByLabel('Project name').fill('Maquette landing');
  await shot(switcher, 'switcher-new-project');
  await switcher.keyboard.press('Escape');
  await projectButton.click();
  await switcher.getByRole('menuitemradio', { name: /Rooftop cinema/ }).click();
  await switcher.getByText('No runs yet').first().waitFor();
  await shot(switcher, 'switcher-board-after');

  const many = await open('&scenario=configured&projects=many', '#/board');
  await many.locator('[aria-haspopup=menu]').first().click();
  const search = many.getByRole('searchbox', { name: 'Find a project' });
  await search.waitFor();
  await search.fill('har');
  await shot(many, 'switcher-search');

  const rail = await open('&scenario=configured', '#/board');
  await rail.getByRole('button', { name: 'Collapse' }).click();
  await rail.locator('[aria-haspopup=menu]').first().click();
  await rail.getByRole('menu', { name: 'Projects' }).waitFor();
  await shot(rail, 'switcher-collapsed-rail');

  const move = await open('&scenario=configured', '#/lab');
  await move.getByText('Earlier directions').waitFor();
  await move.getByRole('button', { name: /^Maquette landing studio/ }).click();
  await move.getByRole('heading', { name: 'Maquette landing studio' }).waitFor();
  await move.getByRole('button', { name: 'More for Maquette landing studio' }).click();
  await move.getByRole('menuitem', { name: /Move to project/ }).click();
  await move.getByRole('menu', { name: /Move Maquette landing studio/ }).waitFor();
  await shot(move, 'lab-move-direction');

  const narrow = await open('&scenario=configured', '#/lab', { width: 1180, height: 820 });
  await narrow.getByText('Earlier directions').waitFor();
  await narrow.getByRole('button', { name: /^Maquette landing studio/ }).click();
  await narrow.getByRole('heading', { name: 'Maquette landing studio' }).waitFor();
  await shot(narrow, 'lab-long-title');

  const starters = await open('&scenario=configured', '#/pipeline');
  await starters.getByText('New starter scripts available').waitFor();
  await shot(starters, 'pipeline-starters');
}

/** Redo from the Inbox and the Credits sheet, on the redo fixture. */
async function redoShots() {
  const fixture = '&scenario=configured&inbox=redo';
  const inbox = await open(fixture, '#/inbox');
  await inbox.getByRole('heading', { name: 'Automatic checks' }).waitFor();
  await inbox.getByRole('button', { name: /^Sakura/ }).click();
  await inbox.waitForTimeout(2500);
  await inbox.getByRole('button', { name: 'More for Sakura' }).click();
  await inbox.getByRole('menu', { name: 'Sakura actions' }).waitFor();
  await shot(inbox, 'redo-inbox-menu');
  await inbox.getByRole('menuitem', { name: 'Redo...' }).click();
  const panel = inbox.getByRole('dialog', { name: 'Redo Sakura' });
  await panel.getByText('About 30 credits').waitFor();
  await shot(inbox, 'redo-send-panel');
  await panel.getByLabel('Texture prompt').scrollIntoViewIfNeeded();
  await shot(inbox, 'redo-send-panel-prompt');
  await panel.getByRole('button', { name: 'Close' }).click();

  await inbox.getByRole('checkbox', { name: 'Select Character' }).click();
  await inbox.getByRole('checkbox', { name: 'Select Sakura' }).click();
  await inbox.getByRole('toolbar', { name: 'Selected models' }).waitFor();
  await shot(inbox, 'redo-inbox-selected');
  await inbox.getByRole('button', { name: 'Redo 2' }).click();
  const batch = inbox.getByRole('dialog', { name: 'Redo 2 models' });
  await batch.getByText('About 115 credits').waitFor();
  await shot(inbox, 'redo-batch');
  await batch.getByRole('button', { name: 'Send 2 to Tripo' }).click();
  await batch.getByRole('alert').waitFor();
  await shot(inbox, 'redo-batch-confirm');

  const board = await open(fixture, '#/board');
  await board.getByRole('button', { name: /3D queue/ }).click();
  await board
    .getByRole('region', { name: '3D queue' })
    .getByRole('button', { name: 'Credits' })
    .click();
  const credits = board.getByRole('dialog', { name: 'Credits' });
  await credits.getByText('1,240').waitFor();
  await shot(board, 'credits');
  await credits
    .getByRole('group', { name: 'Provider' })
    .getByRole('button', { name: 'Meshy' })
    .click();
  await shot(board, 'credits-meshy');
}

try {
  if (only === 'guards') await guardShots();
  if (!only || only === 'ux') await uxShots();
  if (!only) await coreShots();
  if (!only || only === 'handoff') await handoffShots();
  if (!only || only === 'scene') await sceneShots();
  if (!only || only === 'material') await materialShots();
  if (!only || only === 'redo') await redoShots();
} finally {
  await browser.close();
  await vite?.close();
}

console.log(saved.join('\n'));
