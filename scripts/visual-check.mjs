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
 * a Lab direction, a long Lab title and a Refs card with its picture. The
 * pipeline set (scripts/visual-check/pipeline.mjs) covers the Pipeline
 * runner, the site preview and its Stop, and the status bar's server menu. The handoff set covers Make object refs, an
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
 * Credits sheet (all sends, filtered to Meshy). The byhand set covers the
 * By hand card with its subscription cost and export name, an export the
 * Downloads watch copies in, one it asks about and the downloads picker,
 * a close name waiting for a look next to its ref, the asset chooser and
 * its search, the watch in Settings and the Credits sheet's subscription
 * sends. The built set covers built assets on the Board (a stale and a
 * current card, a rebuild, the card menu) and Add built asset. The motion
 * set (scripts/visual-check/motion.mjs) covers the Motion view, the timeline
 * set (scripts/visual-check/timeline.mjs) the Timeline view, the paths set
 * (scripts/visual-check/paths.mjs) the Inspector's Paths editor and the page
 * set (scripts/visual-check/page.mjs) the Page view and the game set
 * (scripts/visual-check/game.mjs) game projects and the library set
 * (scripts/visual-check/library.mjs) the Library view, Add to library and
 * Update from library, and the board set (scripts/visual-check/board.mjs)
 * a card waiting for a By hand model, its grouped menu and Move to submenu,
 * the header's tooltips, Remove from project and Recently removed, and a
 * card with several budget warnings, and the sync set
 * (scripts/visual-check/sync.mjs) a workspace shared with another computer:
 * the lock question, the repo question, the conflict warning and the
 * Settings help, and the about set (scripts/visual-check/about.mjs)
 * Settings > About and the app updater: inert, the header pill, an update
 * downloading and ready. It renders
 * pages to files and never captures the screen.
 *
 *   PLAYWRIGHT=/path/to/playwright/index.mjs OUT_DIR=/tmp/shots pnpm visual
 *
 * ONLY=handoff, ONLY=scene, ONLY=ux, ONLY=material, ONLY=redo, ONLY=byhand, ONLY=built, ONLY=motion,
 * ONLY=timeline, ONLY=paths, ONLY=pipeline, ONLY=page, ONLY=game, ONLY=library, ONLY=board, ONLY=compare, ONLY=sync, ONLY=ship, ONLY=about or ONLY=size
 * renders that set alone (size holds the Board list and the real size and fix-ups), ONLY=guards the questions asked before work is dropped; PORT
 * picks the dev server port.
 *
 * PLAYWRIGHT defaults to the `playwright` package (a dev dependency); point it
 * at another local copy to use that one. OUT_DIR defaults to `.visual/`.
 */
import { mkdir } from 'node:fs/promises';
import { isAbsolute, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { aboutShots } from './visual-check/about.mjs';
import { boardShots } from './visual-check/board.mjs';
import { gameShots } from './visual-check/game.mjs';
import { libraryShots } from './visual-check/library.mjs';
import { compareShots } from './visual-check/compare.mjs';
import { shipShots } from './visual-check/ship.mjs';
import { motionShots } from './visual-check/motion.mjs';
import { pageShots } from './visual-check/page.mjs';
import { pathsShots } from './visual-check/paths.mjs';
import { pipelineShots } from './visual-check/pipeline.mjs';
import { syncShots } from './visual-check/sync.mjs';
import { timelineShots } from './visual-check/timeline.mjs';

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

/**
 * Waits that hang on WebGL work (models loading, a click while a scene
 * renders) get room for a busy machine, as when two suites run at once.
 */
const SLOW = 90_000;

async function open(query = '', hash = '', viewport = { width: 1440, height: 900 }) {
  const context = await browser.newContext({
    viewport,
    deviceScaleFactor: 1,
  });
  context.setDefaultTimeout(SLOW);
  const page = await context.newPage();
  page.on('pageerror', error => console.error(`page error: ${error.message}`));
  await page.goto(`${base}?delay=300${query}${hash}`);
  return page;
}

const cont = page =>
  page.getByRole('button', { name: /^(Continue|Create project|Open .+)$/ }).click();

/** Object refs, batch sends and the By hand hand-back, on the veil gate fixture. */
async function handoffShots() {
  const fixture = '&scenario=configured&handoff=batch&conceptMs=60000';
  const lab = await open(fixture, '#/lab');
  await lab.getByText('Earlier directions').waitFor();
  await lab.getByRole('button', { name: /^Veil gate props 1 image/ }).click();
  await lab.getByRole('heading', { name: 'Veil gate props', exact: true }).waitFor();
  await lab.getByRole('button', { name: 'Make object refs from Concept 01' }).click();
  const field = lab.getByRole('textbox', { name: 'Objects' });
  await field.fill('lantern post, hooded guardian, ice shards, orb shrine, sigil door');
  await field.press('Enter');
  await field.fill('step ring');
  await lab.getByRole('dialog', { name: 'Make object refs' }).waitFor();
  await shot(lab, 'objects-dialog');

  // A long name stays whole with a counter; a chip opens its name and notes.
  await field.fill(
    'Floating ice shards in pale frosted glass with violet rune light and no wireframe'
  );
  await shot(lab, 'objects-long-name');
  await field.fill('');
  await lab.getByRole('button', { name: 'Edit Ice shards' }).click();
  const editor = lab.getByRole('group', { name: 'Edit Ice shards' });
  await editor
    .getByRole('textbox', { name: 'Notes for Codex' })
    .fill('Frosted glass with violet rune light, no wireframe lines, no loose fragments');
  await shot(lab, 'objects-notes');
  await editor.getByRole('button', { name: 'Done' }).click();
  await shot(lab, 'objects-note-chip');

  const arena = await open('&scenario=configured&conceptMs=60000', '#/lab');
  await arena.getByRole('heading', { name: 'Frozen lake arena' }).waitFor();
  await arena.getByRole('button', { name: 'Make object refs from Concept 01' }).click();
  const names = arena.getByRole('textbox', { name: 'Objects' });
  await names.fill('rune pillar, crystal lantern, ice shard, sword');
  await names.press('Enter');
  await arena.getByRole('button', { name: 'Make 4 object refs' }).click();
  await arena.getByRole('heading', { name: 'Frozen lake arena objects' }).waitFor();
  await shot(arena, 'objects-generating');

  const objects = await open(fixture, '#/lab');
  await objects.getByText('Earlier directions').waitFor();
  await objects.getByRole('button', { name: /^Veil gate props objects/ }).click();
  await objects.getByRole('heading', { name: 'Veil gate props objects' }).waitFor();
  await objects.waitForTimeout(400);
  await shot(objects, 'objects-direction');
  await objects.getByRole('button', { name: 'Send to 3D' }).click();
  await objects.getByRole('dialog', { name: 'Send 6 assets to 3D' }).waitFor();
  await objects.waitForTimeout(300);
  await shot(objects, 'batch-by-hand');

  const board = await open(fixture, '#/board');
  await board.getByRole('button', { name: /^Elsheria, v2/ }).waitFor();
  await board.getByRole('button', { name: 'More for Lantern post' }).click();
  await board.getByRole('menuitem', { name: 'Rename...' }).waitFor();
  await shot(board, 'asset-rename-menu');
  await board.getByRole('menuitem', { name: 'Rename...' }).click();
  await board.getByRole('dialog', { name: 'Rename asset' }).waitFor();
  await shot(board, 'asset-rename-dialog');
  await board.keyboard.press('Escape');
  await board.getByRole('checkbox', { name: 'Select Orb shrine' }).click();
  await board
    .getByRole('checkbox', { name: 'Select Crystal lantern' })
    .click({ modifiers: ['Shift'] });
  await board.getByRole('checkbox', { name: 'Select Veil platform' }).click();
  const selection = board.getByRole('toolbar', { name: 'Selected assets' });
  await selection.waitFor();
  await board.mouse.move(0, 0);
  await shot(board, 'board-multi-select');
  await selection.getByRole('button', { name: 'Clear' }).click();
  await board.getByRole('checkbox', { name: 'Select Veil gate' }).click();
  await board
    .getByRole('checkbox', { name: 'Select Guardian statues' })
    .click({ modifiers: ['Shift'] });
  for (const name of ['Hooded guardian', 'Lantern post'])
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
  await batch.getByRole('button', { name: 'Custom settings for Lantern post' }).click();
  const own = batch.getByRole('group', { name: 'Lantern post settings' });
  const faces = own.getByRole('textbox', { name: 'Face limit' });
  await faces.fill('12000');
  await faces.press('Enter');
  await own.getByText('PBR', { exact: true }).click();
  await batch.getByRole('button', { name: 'Custom settings for Hooded guardian' }).click();
  const sculptor = batch.getByRole('group', { name: 'Hooded guardian settings' });
  const sculptFaces = sculptor.getByRole('textbox', { name: 'Face limit' });
  await sculptFaces.fill('2000');
  await sculptFaces.press('Enter');
  await board.waitForTimeout(300);
  await shot(board, 'batch-custom-settings');
  await sculptor.getByRole('button', { name: 'Use batch settings' }).click();
  await batch.getByRole('button', { name: 'Custom settings for Hooded guardian' }).click();
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
  await card.getByRole('button', { name: /^Elsheria, v2/ }).waitFor();
  await card.getByRole('button', { name: 'More for Sigil door' }).click();
  await card.getByRole('menu', { name: 'Sigil door actions' }).waitFor();
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
  await board.getByRole('button', { name: 'More for Guardian statues' }).click();
  await board.getByRole('menuitem', { name: 'Send to 3D' }).click();
  await board.getByRole('dialog', { name: /Send Guardian statues to 3D/ }).waitFor();
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
  await board.getByRole('button', { name: /^Elsheria, v2/ }).waitFor();
  await hoverTip(
    board,
    board.getByRole('checkbox', { name: 'Select Veil platform' }),
    'ux-card-select'
  );
  await board.getByRole('button', { name: 'More for Guardian statues' }).click();
  await board.getByRole('menuitem', { name: 'Send to 3D' }).click();
  const sheet = board.getByRole('dialog', { name: /Send Guardian statues to 3D/ });
  await sheet.waitFor();
  await hoverTip(board, sheet.getByRole('button', { name: 'Close' }), 'ux-sheet-close-edge');

  // An Inbox model that did not come from a send has nothing to redo.
  const inbox = await open('&scenario=configured', '#/inbox');
  await inbox.getByRole('heading', { name: 'Automatic checks' }).waitFor();
  await inbox.getByRole('button', { name: 'More for Elsheria' }).click();
  await hoverTip(
    inbox,
    inbox.getByRole('menuitem', { name: 'Redo...' }),
    'ux-menu-disabled-reason'
  );

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
  await hoverTip(logs, logs.getByRole('button', { name: 'Run pipeline' }), 'ux-pipeline-run');

  const layout = await open('&scenario=configured', '#/inspector');
  await layout.getByRole('tab', { name: 'Layout' }).click();
  await layout.getByRole('heading', { name: 'Scene layout' }).waitFor();
  await layout.getByText(/Loading \d+ models/).waitFor({ state: 'detached', timeout: SLOW });
  await layout.waitForTimeout(1500);
  await hoverTip(layout, layout.getByRole('button', { name: 'Undo' }), 'ux-layout-undo-disabled');
  await hoverTip(
    layout,
    layout.getByRole('button', { name: 'Add stop from view' }),
    'ux-layout-add-stop'
  );
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
  await layout.getByText(/Loading \d+ models/).waitFor({ state: 'detached', timeout: SLOW });
  await layout.waitForTimeout(2500);
  await shot(layout, 'scene-layout');
  const details = layout.getByRole('complementary', { name: 'Layout details' });
  await details.getByRole('button', { name: /^Elsheria/ }).click();
  await layout.waitForTimeout(800);
  await shot(layout, 'scene-layout-model');
  await layout.keyboard.press('e');
  await layout.waitForTimeout(500);
  await shot(layout, 'scene-layout-rotate');
  await details.getByRole('button', { name: /^1 Arrival/ }).click();
  await layout.waitForTimeout(600);
  await shot(layout, 'scene-layout-stop');
  await details.getByRole('button', { name: 'Look through', exact: true }).click();
  await layout.waitForTimeout(1800);
  await shot(layout, 'scene-layout-look');

  const board = await open('&scenario=configured', '#/board');
  await board.getByRole('button', { name: 'More for Elsheria' }).click();
  await board.getByRole('menuitem', { name: 'Copy to project...' }).click();
  await board.getByRole('menu', { name: 'Copy Elsheria to project' }).waitFor();
  await shot(board, 'board-copy-to-project');
  await board.getByRole('menuitem', { name: 'Frost cathedral' }).click();
  await board.getByText('Copied Elsheria to Frost cathedral').waitFor();
  await shot(board, 'board-copied');

  const pipeline = await open('&scenario=idle', '#/pipeline');
  await pipeline.getByText('First view · Arrival').waitFor();
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
  await page.getByText(/WebGL 2 · \d+ fps/).waitFor({ timeout: SLOW });
  await page.waitForTimeout(3500);
  const details = page.getByRole('complementary', { name: 'Model details' });
  await details.getByRole('button', { name: /^Elsheria/ }).click();
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

  await details.getByRole('combobox', { name: 'Applies to' }).selectOption('Elsheria');
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
  await repo.fill('~/Documents/Projects/frostveil-site');
  await repo.press('Enter');
  await page.getByText('Git repo found.').waitFor();
  await shot(page, 'onboarding-6-project');

  await cont(page);
  await page.getByRole('heading', { name: 'Frostveil is ready' }).waitFor();
  await shot(page, 'onboarding-7-ready');

  await cont(page);
  await page.getByRole('heading', { name: 'Your first build in three steps' }).waitFor();
  await shot(page, 'board-empty');

  await page.getByRole('button', { name: 'Pipeline', exact: true }).click();
  await page.getByRole('heading', { name: 'Pipeline runner' }).waitFor();
  await shot(page, 'template-pipeline');

  await page.getByRole('button', { name: 'Settings', exact: true }).click();
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
  await lab.getByRole('heading', { name: 'Frozen lake arena' }).waitFor();
  await shot(lab, 'lab-results');
  await lab.getByRole('button', { name: 'New', exact: true }).click();
  await lab.getByLabel('Direction name').fill('Guardian statue');
  await lab
    .getByLabel('Prompt')
    .fill('A weathered guardian statue on mossy steps, warm candle light, night, soft fog.');
  await lab.getByRole('button', { name: 'Add reference image' }).click();
  await lab.getByRole('button', { name: 'frozen-lake-concept.png' }).click();
  await shot(lab, 'lab-idle');
  const labRunning = await open('&scenario=configured&lab=running', '#/lab');
  await labRunning.getByText('Image queue').waitFor();
  await shot(labRunning, 'lab-running');
  const labLimited = await open('&scenario=configured&lab=limited', '#/lab');
  await labLimited.getByText(/^Waiting, rate limited, retries at /).waitFor();
  await shot(labLimited, 'lab-rate-limited');
  const labUsage = await open('&scenario=configured&lab=usage', '#/lab');
  await labUsage
    .getByText(/^Waiting for the Codex usage limit until about /)
    .first()
    .waitFor();
  await shot(labUsage, 'lab-usage-limit');

  const agents = await open('&scenario=configured&agents=active', '#/settings');
  await agents.getByRole('heading', { name: '3D providers' }).first().waitFor();
  await agents.getByRole('button', { name: 'Agents', exact: true }).click();
  await agents.getByText('Listening on http://127.0.0.1:4791/mcp', { exact: false }).waitFor();
  await agents.waitForTimeout(600);
  await shot(agents, 'settings-agents');
  const prompt = await open('&scenario=configured&agents=active', '#/board');
  await prompt.getByRole('alertdialog', { name: 'Send Rune obelisk to Tripo?' }).waitFor();
  await shot(prompt, 'agents-approval-prompt');

  const board = await open('&scenario=configured', '#/board');
  await board.getByRole('button', { name: /^Elsheria, v2/ }).waitFor();
  await shot(board, 'board');
  await board.getByRole('button', { name: /3D queue/ }).click();
  await board.getByRole('region', { name: '3D queue' }).waitFor();
  await shot(board, 'provider-queue');
  await board.keyboard.press('Escape');
  await board.getByRole('button', { name: 'More for Guardian statues' }).click();
  await board.getByRole('menuitem', { name: 'Send to 3D' }).click();
  await board.getByRole('dialog', { name: /Send Guardian statues to 3D/ }).waitFor();
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
  await inbox.getByRole('button', { name: /^Veil gate/ }).click();
  await inbox.waitForTimeout(2500);
  await shot(inbox, 'inbox-prop');
  const inspector = await open('&scenario=configured', '#/inspector');
  await inspector.getByRole('heading', { name: 'Scene comparison' }).waitFor();
  // Both sides load, the camera reaches the first stop and the strip fills in.
  await inspector.getByText(/WebGL 2 · \d+ fps/).waitFor({ timeout: SLOW });
  await inspector.waitForTimeout(3500);
  await shot(inspector, 'inspector-compare');
  await inspector.getByRole('button', { name: 'Fly to Keystone' }).click();
  await inspector.waitForTimeout(1800);
  await shot(inspector, 'inspector-stop');
  await inspector.getByRole('button', { name: 'Fly to Pullback' }).click();
  await inspector.waitForTimeout(1600);
  await inspector
    .getByRole('group', { name: 'View mode' })
    .getByRole('button', { name: 'Wireframe' })
    .click();
  await inspector.getByRole('button', { name: /^Ice crystals/ }).click();
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
  await switcher.getByRole('menuitemradio', { name: /Frost cathedral/ }).click();
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
  await move.getByRole('button', { name: /^Academy landing/ }).click();
  await move.getByRole('heading', { name: 'Academy landing' }).waitFor();
  await move.getByRole('button', { name: 'More for Academy landing' }).click();
  await move.getByRole('menuitem', { name: /Move to project/ }).click();
  await move.getByRole('menu', { name: /Move Academy landing/ }).waitFor();
  await shot(move, 'lab-move-direction');

  const narrow = await open('&scenario=configured', '#/lab', { width: 1180, height: 820 });
  await narrow.getByText('Earlier directions').waitFor();
  await narrow.getByRole('button', { name: /^Academy landing/ }).click();
  await narrow.getByRole('heading', { name: 'Academy landing' }).waitFor();
  await shot(narrow, 'lab-long-title');
}

/** Redo from the Inbox and the Credits sheet, on the redo fixture. */
async function redoShots() {
  const fixture = '&scenario=configured&inbox=redo';
  const inbox = await open(fixture, '#/inbox');
  await inbox.getByRole('heading', { name: 'Automatic checks' }).waitFor();
  await inbox.getByRole('button', { name: /^Veil gate/ }).click();
  await inbox.waitForTimeout(2500);
  await inbox.getByRole('button', { name: 'More for Veil gate' }).click();
  await inbox.getByRole('menu', { name: 'Veil gate actions' }).waitFor();
  await shot(inbox, 'redo-inbox-menu');
  await inbox.getByRole('menuitem', { name: 'Redo...' }).click();
  const panel = inbox.getByRole('dialog', { name: 'Redo Veil gate' });
  await panel.getByText('About 30 credits').waitFor();
  await shot(inbox, 'redo-send-panel');
  await panel.getByLabel('Texture prompt').scrollIntoViewIfNeeded();
  await shot(inbox, 'redo-send-panel-prompt');
  await panel.getByRole('button', { name: 'Close' }).click();

  await inbox.getByRole('checkbox', { name: 'Select Elsheria' }).click();
  await inbox.getByRole('checkbox', { name: 'Select Veil gate' }).click();
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

// Size group, Board list: the list layout, sorted, and with a selection.
async function listShots() {
  const board = await open('&scenario=configured&handoff=batch', '#/board');
  await board.getByRole('button', { name: /^Elsheria, v2/ }).waitFor();
  await board
    .getByRole('group', { name: 'Board layout' })
    .getByRole('button', { name: 'List' })
    .click();
  const grid = board.getByRole('grid', { name: 'Assets' });
  await grid.waitFor();
  await board.mouse.move(0, 0);
  await shot(board, 'board-list');
  await board
    .getByRole('columnheader', { name: /^Tris$/ })
    .getByRole('button')
    .click();
  await board.getByRole('checkbox', { name: 'Select Elsheria' }).click();
  await board
    .getByRole('checkbox', { name: 'Select Observatory arch' })
    .click({ modifiers: ['Shift'] });
  await board.getByRole('toolbar', { name: 'Selected assets' }).waitFor();
  await grid.getByRole('row', { name: /^Crystal lantern$/ }).focus();
  await board.mouse.move(0, 0);
  await shot(board, 'board-list-selected');
  await board.keyboard.press('Enter');
  await board.getByRole('menu', { name: 'Crystal lantern actions' }).waitFor();
  await shot(board, 'board-list-menu');
  await board.keyboard.press('Escape');
  await board
    .getByRole('group', { name: 'Stage filter' })
    .getByRole('button', { name: 'Tripo' })
    .click();
  await board.getByRole('searchbox', { name: 'Search assets' }).fill('cup');
  await board.mouse.move(0, 0);
  await shot(board, 'board-list-filtered');
}
// End of the Board list shots.

/** Built assets: a stale and a current card, a rebuild, the card menu and Add built asset. */
async function builtShots() {
  const board = await open('&scenario=configured&built=demo', '#/board');
  await board.getByRole('button', { name: /^Newtons cradle, v1/ }).waitFor();
  await board.getByText('Script changed').waitFor();
  await board.mouse.move(0, 0);
  await shot(board, 'built-board');
  await board.getByRole('button', { name: 'Rebuild' }).click();
  await board.getByText('Building').first().waitFor();
  await shot(board, 'built-building');
  await board.getByText('Built Newtons cradle v2').waitFor();
  await board.mouse.move(0, 0);
  await shot(board, 'built-rebuilt');
  await board.getByRole('button', { name: 'More for Laptop' }).click();
  await board.getByRole('menu', { name: 'Laptop actions' }).waitFor();
  await shot(board, 'built-menu');
  await board.keyboard.press('Escape');
  await board.getByRole('button', { name: 'Built asset' }).click();
  const dialog = board.getByRole('dialog', { name: 'Add built asset' });
  await dialog.waitFor();
  await shot(board, 'built-add-empty');
  await dialog.getByRole('button', { name: 'Choose...' }).click();
  await dialog.getByRole('button', { name: 'Add files...' }).click();
  await dialog.getByRole('list', { name: 'Helpers' }).waitFor();
  await shot(board, 'built-add-filled');
  await dialog.getByRole('button', { name: 'Add and build' }).click();
  await dialog.waitFor({ state: 'detached' });
  await board.getByText('Built Desk v1').waitFor();
  await board.mouse.move(0, 0);
  await shot(board, 'built-added');
}

// Size group, real size and fix-ups: the Inbox controls, a fix on a check,
// the preview numbers, the Board dialog and the Inspector's Fix-ups list.
async function prepShots() {
  const inbox = await open('&scenario=configured', '#/inbox', { width: 1440, height: 1300 });
  await inbox.getByRole('heading', { name: 'Automatic checks' }).waitFor();
  await inbox.getByRole('button', { name: /^Elsheria/ }).click();
  const checks = inbox.getByRole('complementary', { name: 'Automatic checks' });
  await checks.getByRole('button', { name: 'Decimate to 60k' }).waitFor();
  await inbox.mouse.move(0, 0);
  await shot(checks, 'inbox-fix-on-check');
  await checks.getByRole('button', { name: 'Decimate to 60k' }).click();
  await checks.getByRole('button', { name: /^Suggested: 170 cm high/ }).click();
  const prep = checks.getByRole('region', { name: 'Real size and fix-ups' });
  await prep.getByText(/^Scaled/).waitFor();
  await prep.scrollIntoViewIfNeeded();
  await inbox.mouse.move(0, 0);
  await shot(checks, 'inbox-real-size');
  await prep.getByRole('button', { name: 'Preview the numbers' }).click();
  await prep.getByRole('table', { name: 'Before and after the fix-ups' }).waitFor();
  await prep.getByRole('table').scrollIntoViewIfNeeded();
  await inbox.mouse.move(0, 0);
  await shot(checks, 'inbox-fix-preview');
  await shot(inbox, 'inbox-real-size-page');

  const board = await open('&scenario=configured', '#/board');
  await board.getByRole('button', { name: 'More for Elsheria' }).click();
  await board.getByRole('menuitem', { name: 'Real size and fix-ups...' }).click();
  const dialog = board.getByRole('dialog', { name: 'Real size and fix-ups' });
  await dialog.getByText(/^Measured/).waitFor();
  await dialog.getByRole('button', { name: /^Suggested: 170 cm high/ }).click();
  await dialog.getByRole('button', { name: 'Origin to bottom centre' }).click();
  await board.mouse.move(0, 0);
  await shot(board, 'board-real-size-dialog');
  await dialog.getByRole('button', { name: 'Save' }).click();
  await dialog.waitFor({ state: 'detached' });

  const page = await open('&scenario=configured', '#/inspector', { width: 1440, height: 1700 });
  await page.getByRole('heading', { name: 'Scene comparison' }).waitFor();
  const details = page.getByRole('complementary', { name: 'Model details' });
  await details.getByRole('button', { name: /^Elsheria/ }).click();
  const panel = details.getByRole('region', { name: 'Real size and fix-ups' });
  await panel.getByText(/^Measured/).waitFor();
  await panel.getByRole('button', { name: 'Decimate' }).click();
  await panel.getByRole('button', { name: 'Shrink textures' }).click();
  await panel.getByRole('button', { name: 'Preview the numbers' }).click();
  await panel.getByRole('table', { name: 'Before and after the fix-ups' }).waitFor();
  await panel.scrollIntoViewIfNeeded();
  await page.mouse.move(0, 0);
  await shot(panel, 'inspector-fixups');
  await shot(details, 'inspector-model-details');
}

/**
 * The By hand flow: the settings card with its subscription cost and export
 * name, an export the Downloads watch copies in by itself, one it asks about
 * ("Which asset is this?") and the asset chooser, a batch export whose name
 * is only close to a send ("Needs a look"), the watch in Settings and the
 * Credits sheet's subscription sends.
 */
async function byHandShots() {
  const fixture = '&scenario=configured&handoff=batch';
  const board = await open(fixture, '#/board');
  await board.getByRole('button', { name: /^By hand \d+$/ }).click();
  const checklist = board.getByRole('dialog', { name: 'By hand checklist' });
  await checklist.getByRole('button', { name: 'Settings card' }).first().click();
  await checklist.getByText('credits', { exact: false }).first().waitFor();
  await shot(board, 'byhand-card');
  await checklist.getByText('Bring the GLB back', { exact: true }).scrollIntoViewIfNeeded();
  await shot(board, 'byhand-card-export-name');

  const named = await open(`${fixture}&arrival=named`, '#/board');
  await named.getByText('is in the Inbox', { exact: false }).first().waitFor();
  await shot(named, 'byhand-arrival-imported');

  const asking = await open(`${fixture}&arrival=unmatched`, '#/board');
  await asking.getByText('A new model in Downloads').waitFor();
  await shot(asking, 'byhand-arrival-asking');
  await asking.getByRole('button', { name: 'Choose' }).click();
  const picker = asking.getByRole('dialog', { name: 'New downloads' });
  await picker.getByText('model (3).glb').waitFor();
  await shot(asking, 'byhand-picker');
  await picker.getByRole('button', { name: 'Attach to an asset' }).click();
  const chooser = asking.getByRole('dialog', { name: 'Attach to an asset' });
  await chooser.getByRole('option').first().waitFor();
  await shot(asking, 'byhand-picker-chosen');

  // A Tripo batch export: one name close to a send waits for a look, two fit none.
  const look = await open(`${fixture}&arrival=look`, '#/board');
  await look.getByText('Is this Lantern post?').first().waitFor();
  await shot(look, 'byhand-arrival-look');
  await look.getByRole('button', { name: 'Look' }).click();
  const looking = look.getByRole('dialog', { name: 'New downloads' });
  await looking.getByRole('img', { name: 'The model in lantern post 3d model.glb' }).waitFor();
  await shot(look, 'byhand-picker-look');
  await looking.getByRole('button', { name: 'Pick another asset' }).click();
  const another = look.getByRole('dialog', { name: 'Attach to an asset' });
  await another.getByRole('option').first().waitFor();
  await shot(look, 'byhand-chooser');
  await another.getByRole('combobox').fill('step');
  await shot(look, 'byhand-chooser-search');

  const settings = await open(fixture, '#/settings');
  const watch = settings.getByRole('switch', { name: /Watch for By hand exports/ });
  await watch.scrollIntoViewIfNeeded();
  await settings.getByText(/^Watching for \d+ By hand send/).waitFor();
  await shot(settings.locator('#settings-providers'), 'byhand-settings-watch');
  await watch.click({ force: true });
  await settings.getByText(/^Off\. Use Add model/).waitFor();
  await shot(settings.locator('#settings-providers'), 'byhand-settings-watch-off');

  const credits = await open(fixture, '#/board');
  await credits.getByRole('button', { name: /3D queue/ }).click();
  await credits
    .getByRole('region', { name: '3D queue' })
    .getByRole('button', { name: 'Credits' })
    .click();
  const sheet = credits.getByRole('dialog', { name: 'Credits' });
  await sheet
    .getByRole('group', { name: 'Paid with' })
    .getByRole('button', { name: 'Subscription' })
    .click();
  await sheet.getByText('Subscription credits are estimates', { exact: false }).waitFor();
  await shot(credits, 'byhand-credits-subscription');
}

try {
  if (only === 'guards') await guardShots();
  if (!only || only === 'ux') await uxShots();
  if (!only) await coreShots();
  if (!only || only === 'handoff') await handoffShots();
  if (!only || only === 'scene') await sceneShots();
  if (!only || only === 'material') await materialShots();
  if (!only || only === 'redo') await redoShots();
  if (!only || only === 'byhand') await byHandShots();
  if (!only || only === 'size') await listShots();
  if (!only || only === 'size') await prepShots();
  if (!only || only === 'built') await builtShots();
  if (!only || only === 'board') await boardShots({ open, shot });
  if (!only || only === 'motion') await motionShots({ open, shot });
  if (!only || only === 'timeline') await timelineShots({ open, shot });
  if (!only || only === 'paths') await pathsShots({ open, shot });
  if (!only || only === 'pipeline') await pipelineShots({ open, shot, hoverTip });
  if (!only || only === 'page') await pageShots({ open, shot });
  if (!only || only === 'game') await gameShots({ open, shot });
  if (!only || only === 'library') await libraryShots({ open, shot });
  if (!only || only === 'compare') await compareShots({ open, shot });
  if (!only || only === 'ship') await shipShots({ open, shot, hoverTip });
  if (!only || only === 'sync') await syncShots({ open, shot });
  if (!only || only === 'about') await aboutShots({ open, shot });
} finally {
  await browser.close();
  await vite?.close();
}

console.log(saved.join('\n'));
