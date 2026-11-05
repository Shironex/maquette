/**
 * The views and layers scripts/a11y-check.mjs runs axe over: every view of
 * mock mode and its main dialogs and sheets. Each case opens a page, gets it
 * to the state worth checking and resolves once that state shows.
 */

const CONFIGURED = '&scenario=configured';

const boardReady = page => page.getByRole('button', { name: /^Elsheria, v2/ }).waitFor();

async function cardMenu(page, asset, item) {
  await page.getByRole('button', { name: `More for ${asset}` }).click();
  await page.getByRole('menuitem', { name: item }).click();
}

async function partsTab(page, model) {
  await page.getByRole('tab', { name: 'Parts' }).click();
  await page.getByRole('combobox', { name: 'Selected model' }).selectOption({ label: model });
}

async function openQueue(page) {
  await boardReady(page);
  await page.getByRole('button', { name: 'More board actions' }).click();
  await page.getByRole('menuitem', { name: /^All sends/ }).click();
  await page
    .getByRole('region', { name: 'Sends' })
    .getByRole('button', { name: 'Credits' })
    .click();
  await page.getByRole('dialog', { name: 'Credits' }).getByText('1,240').waitFor();
}

async function openCameraPlan(page) {
  await page.getByRole('region', { name: 'Pinned' }).getByRole('button').click();
  await page.getByRole('region', { name: 'Order after Apply' }).getByText('Arrival').waitFor();
}

/** Opens a canvas from the list's table (the rail lists the recent ones by name too). */
const openCanvas = (page, name) =>
  page
    .getByRole('region', { name: 'All canvases' })
    .getByRole('button', { name, exact: true })
    .click();

const cameraInvoke = (page, cmd) =>
  page.evaluate(c => window.__TAURI_INTERNALS__.invoke(c, { project: 'frostveil' }), cmd);

/** `{ name, query, hash, ready(page) }`, in the order the views sit on the rail. */
export const cases = [
  {
    name: 'onboarding welcome',
    query: '',
    ready: page => page.getByRole('button', { name: 'Set up' }).waitFor(),
  },
  {
    name: 'onboarding workspace',
    query: '',
    async ready(page) {
      await page.getByRole('button', { name: 'Set up' }).click();
      await page.getByText('Empty folder, ready to use').waitFor();
    },
  },
  {
    name: 'onboarding toolchain',
    query: '&scenario=no-blender',
    async ready(page) {
      await page.getByRole('button', { name: 'Set up' }).click();
      await page.getByText('Empty folder, ready to use').waitFor();
      await page.getByRole('button', { name: 'Continue' }).click();
      await page.getByText('Blender is required.').waitFor();
    },
  },
  { name: 'board', query: CONFIGURED, hash: '#/board', ready: boardReady },
  {
    name: 'first run card',
    query: `${CONFIGURED}&game=import`,
    hash: '#/board',
    ready: page => page.getByRole('region', { name: 'Your first build in three steps' }).waitFor(),
  },
  ...[
    ['missing', 'no-blender', 'Blender is missing'],
    ['did not run', 'no-blender-run', 'Blender did not run'],
    ['saved path gone', 'no-blender-path', 'Your saved Blender path is gone'],
  ].map(([name, state, title]) => ({
    name: `settings blender ${name}`,
    query: `${CONFIGURED}&render=${state}`,
    hash: '#/settings',
    ready: page => page.getByRole('region', { name: title }).waitFor(),
  })),
  {
    name: 'board list',
    query: `${CONFIGURED}&handoff=batch`,
    hash: '#/board',
    async ready(page) {
      await boardReady(page);
      await page
        .getByRole('group', { name: 'Board layout' })
        .getByRole('button', { name: 'List' })
        .click();
      await page.getByRole('checkbox', { name: 'Select Veil gate' }).first().waitFor();
    },
  },
  {
    name: 'board card menu',
    query: CONFIGURED,
    hash: '#/board',
    async ready(page) {
      await boardReady(page);
      await page.getByRole('button', { name: 'More for Guardian statues' }).click();
      await page.getByRole('menuitem', { name: 'Send to 3D' }).waitFor();
    },
  },
  {
    name: 'send to 3D',
    query: CONFIGURED,
    hash: '#/board',
    async ready(page) {
      await boardReady(page);
      await cardMenu(page, 'Guardian statues', 'Send to 3D');
      await page.getByRole('dialog', { name: /Send Guardian statues to 3D/ }).waitFor();
    },
  },
  {
    name: 'batch send',
    query: `${CONFIGURED}&handoff=batch`,
    hash: '#/board',
    async ready(page) {
      await boardReady(page);
      for (const name of ['Veil gate', 'Guardian statues', 'Lantern post'])
        await page.getByRole('checkbox', { name: `Select ${name}` }).click();
      await page
        .getByRole('toolbar', { name: 'Selected assets' })
        .getByRole('button', { name: 'Send to 3D' })
        .click();
      await page.getByRole('dialog', { name: /^Send \d+ assets to 3D$/ }).waitFor();
    },
  },
  {
    name: 'refs sheet',
    query: `${CONFIGURED}&lab=refs`,
    hash: '#/board',
    async ready(page) {
      await boardReady(page);
      await cardMenu(page, 'Guardian statues', 'Refs and views...');
      await page.getByRole('dialog', { name: 'Guardian statues refs' }).waitFor();
    },
  },
  {
    name: 'new downloads and asset chooser',
    query: `${CONFIGURED}&handoff=batch&arrival=unmatched`,
    hash: '#/board',
    async ready(page) {
      await page.getByText('A new model in Downloads').waitFor();
      await page.getByRole('button', { name: 'Choose' }).click();
      const picker = page.getByRole('dialog', { name: 'New downloads' });
      await picker.getByText('model (3).glb').waitFor();
      await picker.getByRole('button', { name: 'Attach to an asset' }).click();
      await page
        .getByRole('dialog', { name: 'Attach to an asset' })
        .getByRole('option')
        .first()
        .waitFor();
    },
  },
  {
    name: 'new downloads',
    query: `${CONFIGURED}&handoff=batch&arrival=look`,
    hash: '#/board',
    async ready(page) {
      await page.getByText('Is this Lantern post?').first().waitFor();
      await page.getByRole('button', { name: 'Look' }).click();
      await page.getByRole('dialog', { name: 'New downloads' }).waitFor();
    },
  },
  {
    name: 'credits',
    query: `${CONFIGURED}&inbox=redo`,
    hash: '#/board',
    ready: openQueue,
  },
  {
    name: 'plan list',
    query: `${CONFIGURED}&plans=demo`,
    hash: '#/plan',
    ready: page => page.getByRole('region', { name: 'All canvases' }).waitFor(),
  },
  {
    name: 'plan canvas',
    query: `${CONFIGURED}&plans=demo`,
    hash: '#/plan',
    async ready(page) {
      await openCanvas(page, 'Combat ideas');
      await page.getByRole('group', { name: 'Note: Frost veil' }).waitFor();
    },
  },
  {
    name: 'plan link dialog',
    query: `${CONFIGURED}&plans=demo`,
    hash: '#/plan',
    async ready(page) {
      await openCanvas(page, 'Combat ideas');
      await page.getByRole('button', { name: /^Link/ }).click();
      await page.getByRole('dialog', { name: 'Add a link' }).waitFor();
    },
  },
  {
    name: 'plan new canvas dialog',
    query: `${CONFIGURED}&plans=demo`,
    hash: '#/plan',
    async ready(page) {
      await page.getByRole('button', { name: 'New canvas' }).first().click();
      const dialog = page.getByRole('dialog', { name: 'New canvas' });
      await dialog
        .locator('label')
        .filter({ hasText: /^Story/ })
        .click();
    },
  },
  {
    name: 'plan empty list presets',
    query: `${CONFIGURED}&plans=none`,
    hash: '#/plan',
    ready: page => page.getByRole('radiogroup', { name: 'Start from' }).waitFor(),
  },
  {
    name: 'plan empty canvas',
    query: `${CONFIGURED}&plans=demo`,
    hash: '#/plan',
    async ready(page) {
      await openCanvas(page, 'Boss arena');
      await page.getByRole('region', { name: 'Or ask an agent to start it' }).waitFor();
    },
  },
  {
    name: 'plan recent canvases on the rail',
    query: `${CONFIGURED}&plans=demo`,
    hash: '#/plan',
    async ready(page) {
      const recent = page.getByRole('list', { name: 'Recent canvases' });
      await recent.getByRole('button', { name: 'Combat ideas' }).click();
      await recent.locator('[aria-current="page"]').waitFor();
      await page.getByRole('group', { name: 'Note: Frost veil' }).waitFor();
    },
  },
  {
    name: 'plan asset links',
    query: `${CONFIGURED}&plans=demo`,
    hash: '#/plan',
    async ready(page) {
      await openCanvas(page, 'Boss: Frost Warden');
      await page.getByRole('group', { name: 'Asset: Asset not found' }).waitFor();
    },
  },
  {
    name: 'plan asset picker',
    query: `${CONFIGURED}&plans=demo`,
    hash: '#/plan',
    async ready(page) {
      await openCanvas(page, 'Boss: Frost Warden');
      await page.getByRole('button', { name: /^Asset/ }).click();
      await page
        .getByRole('dialog', { name: 'Link assets' })
        .getByText('Frozen lake arena')
        .waitFor();
    },
  },
  {
    name: 'plan agent write and changes',
    query: `${CONFIGURED}&plans=demo`,
    hash: '#/plan',
    async ready(page) {
      await openCanvas(page, 'Combat ideas');
      await page.getByRole('group', { name: 'Note: Frost veil' }).waitFor();
      await page.waitForTimeout(50);
      await page.evaluate(() =>
        window.__TAURI_INTERNALS__.invoke('mock_plan_agent', {
          project: 'frostveil',
          id: 'combat-ideas',
        })
      );
      await page.getByText('Research agent changed the canvas').waitFor();
      await page.evaluate(() =>
        window.__TAURI_INTERNALS__.invoke('mock_plan_agent', {
          project: 'frostveil',
          id: 'combat-ideas',
          edit: 'n-02',
        })
      );
      await page.getByRole('button', { name: /^Changes/ }).click();
      await page.getByRole('button', { name: 'Keep run' }).waitFor();
    },
  },
  {
    name: 'plan outside edit',
    query: `${CONFIGURED}&plans=demo`,
    hash: '#/plan',
    async ready(page) {
      await openCanvas(page, 'Combat ideas');
      await page.getByRole('group', { name: 'Note: Frost veil' }).dblclick();
      await page.getByRole('textbox', { name: 'Note text' }).fill('Mine, not saved yet');
      await page.keyboard.press('Control+Enter');
      await page.evaluate(() =>
        window.__TAURI_INTERNALS__.invoke('mock_plan_outside', {
          project: 'frostveil',
          id: 'combat-ideas',
          node: 'n-01',
          text: 'Changed on the laptop',
        })
      );
      await page.getByRole('dialog', { name: 'The canvas changed outside' }).waitFor();
    },
  },
  {
    name: 'plan clash left unanswered',
    query: `${CONFIGURED}&plans=demo`,
    hash: '#/plan',
    async ready(page) {
      await openCanvas(page, 'Combat ideas');
      await page.getByRole('group', { name: 'Note: Frost veil' }).dblclick();
      await page.getByRole('textbox', { name: 'Note text' }).fill('Mine, not saved yet');
      await page.keyboard.press('Control+Enter');
      await page.evaluate(() =>
        window.__TAURI_INTERNALS__.invoke('mock_plan_outside', {
          project: 'frostveil',
          id: 'combat-ideas',
          node: 'n-01',
          text: 'Changed on the laptop',
        })
      );
      await page.getByRole('dialog', { name: 'The canvas changed outside' }).waitFor();
      await page.keyboard.press('Escape');
      await page.getByRole('button', { name: 'Answer clash' }).waitFor();
    },
  },
  {
    name: 'plan removal proposal in changes',
    query: `${CONFIGURED}&plans=demo`,
    hash: '#/plan',
    async ready(page) {
      await openCanvas(page, 'Combat ideas');
      await page.getByRole('group', { name: 'Note: Frost veil' }).waitFor();
      await page.evaluate(() =>
        window.__TAURI_INTERNALS__.invoke('mock_plan_propose', {
          project: 'frostveil',
          id: 'combat-ideas',
          nodes: ['n-06'],
          reason: 'The Frost veil card answers it.',
        })
      );
      await page.getByText('asked to remove').waitFor();
      await page.getByRole('button', { name: /^Changes/ }).click();
      await page.getByRole('button', { name: 'Keep it' }).waitFor();
    },
  },
  {
    name: 'plan overlaps panel',
    query: `${CONFIGURED}&plans=systems`,
    hash: '#/plan',
    async ready(page) {
      await openCanvas(page, 'Combat systems');
      await page.getByRole('button', { name: /^Overlaps/ }).click();
      await page.getByRole('complementary', { name: 'Overlaps' }).waitFor();
      await page.getByText('Same key as', { exact: false }).first().waitFor();
    },
  },
  {
    name: 'plan node toolbar and auto layout off',
    query: `${CONFIGURED}&plans=demo`,
    hash: '#/plan',
    async ready(page) {
      await openCanvas(page, 'Combat ideas');
      await page.getByRole('button', { name: /^Auto layout/ }).click();
      await page.getByRole('button', { name: /^Auto layout/, pressed: false }).waitFor();
      await page.getByRole('group', { name: 'Note: Frost veil' }).click();
      await page.getByRole('button', { name: 'Start a canvas from this' }).waitFor();
    },
  },
  {
    name: 'camera plan',
    query: `${CONFIGURED}&plans=demo`,
    hash: '#/plan',
    async ready(page) {
      await openCameraPlan(page);
      await cameraInvoke(page, 'mock_camera_agent');
      await page.getByRole('group', { name: 'Proposal by Camera agent' }).waitFor();
    },
  },
  {
    name: 'camera plan apply review',
    query: `${CONFIGURED}&plans=demo`,
    hash: '#/plan',
    async ready(page) {
      await openCameraPlan(page);
      await cameraInvoke(page, 'mock_camera_agent');
      await page.getByRole('button', { name: 'Use this order' }).click();
      await page.getByRole('region', { name: 'Order after Apply' }).getByText('new stop').waitFor();
      await page.getByRole('button', { name: 'Apply to layout.json' }).click();
      await page.getByRole('dialog', { name: 'Apply to layout.json' }).waitFor();
    },
  },
  {
    name: 'camera plan with no stops',
    query: `${CONFIGURED}&plans=demo`,
    hash: '#/plan',
    async ready(page) {
      await page.getByRole('region', { name: 'Pinned' }).waitFor();
      await cameraInvoke(page, 'mock_layout_bare');
      await page.getByRole('region', { name: 'Pinned' }).getByRole('button').click();
      await page.getByRole('region', { name: 'No camera stops yet' }).waitFor();
      await page.getByText(/^No stops yet/).waitFor();
    },
  },
  {
    name: 'camera plan behind layout and compare',
    query: `${CONFIGURED}&plans=demo`,
    hash: '#/plan',
    async ready(page) {
      await openCameraPlan(page);
      await cameraInvoke(page, 'mock_layout_outside');
      await page.getByText('layout.json changed since this plan was based on it').waitFor();
      await page.getByRole('button', { name: 'Compare' }).click();
      await page.getByRole('dialog', { name: 'layout.json changed' }).waitFor();
    },
  },
  {
    name: 'inbox',
    query: CONFIGURED,
    hash: '#/inbox',
    ready: page => page.getByRole('heading', { name: 'Automatic checks' }).waitFor(),
  },
  {
    name: 'lab',
    query: `${CONFIGURED}&conceptMs=60000`,
    hash: '#/lab',
    ready: page => page.getByRole('heading', { name: 'Frozen lake arena' }).waitFor(),
  },
  {
    name: 'lab running',
    query: `${CONFIGURED}&lab=running`,
    hash: '#/lab',
    ready: page => page.getByText('Image queue').waitFor(),
  },
  {
    name: 'style board',
    query: `${CONFIGURED}&style=board&conceptMs=60000`,
    hash: '#/lab',
    async ready(page) {
      await page.getByRole('button', { name: 'Open the style board' }).click();
      await page.getByRole('dialog', { name: 'Style board' }).waitFor();
    },
  },
  {
    name: 'pipeline',
    query: CONFIGURED,
    hash: '#/pipeline',
    ready: page => page.getByRole('heading', { name: 'Pipeline runner' }).waitFor(),
  },
  {
    name: 'pipeline running',
    query: '&scenario=running',
    hash: '#/pipeline',
    ready: page => page.getByRole('heading', { name: 'Pipeline runner' }).waitFor(),
  },
  {
    name: 'inspector compare',
    query: CONFIGURED,
    hash: '#/inspector',
    ready: page => page.getByRole('heading', { name: 'Scene comparison' }).waitFor(),
  },
  {
    name: 'inspector layout',
    query: CONFIGURED,
    hash: '#/inspector',
    async ready(page) {
      await page.getByRole('tab', { name: 'Layout' }).click();
      await page.getByRole('tab', { name: 'Layout', selected: true }).waitFor();
    },
  },
  {
    name: 'inspector paths',
    query: `${CONFIGURED}&paths=demo`,
    hash: '#/inspector',
    async ready(page) {
      await page.getByRole('tab', { name: 'Paths' }).click();
      await page.getByRole('tab', { name: 'Paths', selected: true }).waitFor();
    },
  },
  {
    name: 'split sheet',
    query: `${CONFIGURED}&parts=demo`,
    hash: '#/inspector',
    async ready(page) {
      await partsTab(page, 'Rune chest');
      await page.getByRole('button', { name: 'Split into parts...' }).click();
      await page
        .getByRole('dialog', { name: 'Split Rune chest into parts' })
        .getByText(/v1 from Meshy/)
        .waitFor();
    },
  },
  {
    name: 'assemble',
    query: `${CONFIGURED}&parts=demo`,
    hash: '#/inspector',
    async ready(page) {
      await partsTab(page, 'Sigil gate frame');
      await page.getByRole('button', { name: 'Assemble a prop...' }).click();
      await page.getByRole('dialog', { name: 'Assemble one prop' }).waitFor();
    },
  },
  {
    name: 'motion',
    query: `${CONFIGURED}&library=demo&motion=demo`,
    hash: '#/motion',
    ready: page => page.getByRole('button', { name: 'Add clip' }).waitFor(),
  },
  {
    name: 'use on model steps',
    query: `${CONFIGURED}&library=demo&motion=demo`,
    hash: '#/motion',
    async ready(page) {
      await page.getByRole('button', { name: 'Add clip' }).click();
      await page.getByRole('menuitem', { name: 'Add from library' }).click();
      await page.getByRole('dialog', { name: /^Add library clips to / }).waitFor();
    },
  },
  {
    name: 'timeline',
    query: `${CONFIGURED}&timeline=demo`,
    hash: '#/timeline',
    ready: page => page.getByRole('main').waitFor(),
  },
  {
    name: 'page',
    query: `${CONFIGURED}&page=running`,
    hash: '#/page',
    ready: page => page.getByRole('main').waitFor(),
  },
  {
    name: 'library models',
    query: `${CONFIGURED}&library=demo`,
    hash: '#/library',
    ready: page => page.getByRole('main').waitFor(),
  },
  {
    name: 'library animations',
    query: `${CONFIGURED}&library=demo`,
    hash: '#/animations',
    ready: page => page.getByRole('button', { name: 'Import clips' }).waitFor(),
  },
  {
    name: 'settings',
    query: `${CONFIGURED}&agents=active`,
    hash: '#/settings',
    ready: page => page.getByText('Changes save as you go').waitFor(),
  },
  {
    name: 'settings agents tool groups',
    query: `${CONFIGURED}&agents=active`,
    hash: '#/settings',
    async ready(page) {
      const groups = page.getByRole('region', { name: 'Tool groups' });
      await groups
        .getByRole('switch', { name: 'Avatar and rig tools' })
        .locator('xpath=..')
        .click();
      await groups.getByText('140 of 147 tools offered').waitFor();
    },
  },
  {
    name: 'rename asset',
    query: CONFIGURED,
    hash: '#/board',
    async ready(page) {
      await boardReady(page);
      await cardMenu(page, 'Guardian statues', 'Rename...');
      await page.getByRole('dialog', { name: 'Rename asset' }).waitFor();
    },
  },
  {
    name: 'new project',
    query: CONFIGURED,
    hash: '#/board',
    async ready(page) {
      await boardReady(page);
      await page.locator('[aria-haspopup=menu]').first().click();
      await page.getByRole('menuitem', { name: 'New project' }).click();
      await page.getByRole('dialog', { name: 'New project' }).waitFor();
    },
  },
  {
    name: 'game import',
    query: `${CONFIGURED}&game=import`,
    hash: '#/board',
    async ready(page) {
      await page.getByRole('button', { name: 'More ways to add' }).click();
      await page.getByRole('menuitem', { name: 'Import models' }).click();
      await page.getByRole('dialog', { name: /^Import models from / }).waitFor();
    },
  },
  {
    name: 'project switcher',
    query: `${CONFIGURED}&projects=many`,
    hash: '#/board',
    async ready(page) {
      await boardReady(page);
      await page.locator('[aria-haspopup=menu]').first().click();
      await page.getByRole('searchbox', { name: 'Find a project' }).waitFor();
    },
  },
  {
    name: 'remove project refused',
    query: CONFIGURED,
    hash: '#/board',
    async ready(page) {
      await boardReady(page);
      await page.locator('[aria-haspopup=menu]').first().click();
      await page.getByRole('menuitem', { name: 'Remove Frostveil' }).click();
      const dialog = page.getByRole('dialog', { name: 'Remove Frostveil?' });
      await dialog.getByRole('note').waitFor();
      await dialog.locator('button[aria-disabled="true"]', { hasText: 'Remove project' }).waitFor();
    },
  },
  {
    name: 'removed projects',
    query: CONFIGURED,
    hash: '#/board',
    async ready(page) {
      await boardReady(page);
      const switcher = page.locator('[aria-haspopup=menu]').first();
      await switcher.click();
      await page.getByRole('menuitemradio', { name: /^Elsheria showcase/ }).click();
      await page.getByRole('heading', { name: 'Elsheria showcase' }).waitFor();
      await switcher.click();
      await page.getByRole('menuitem', { name: 'Remove Elsheria showcase' }).click();
      const ask = page.getByRole('dialog', { name: 'Remove Elsheria showcase?' });
      await ask.getByRole('button', { name: 'Remove project' }).click();
      await ask.waitFor({ state: 'detached' });
      await switcher.click();
      await page.getByRole('menuitem', { name: 'Removed projects' }).click();
      await page.getByRole('button', { name: 'Restore Elsheria showcase' }).waitFor();
    },
  },
  {
    name: 'plan overlaps all marked as fine',
    query: `${CONFIGURED}&plans=systems`,
    hash: '#/plan',
    async ready(page) {
      await openCanvas(page, 'Combat systems');
      await page.getByRole('button', { name: /^Overlaps/ }).click();
      const panel = page.getByRole('complementary', { name: 'Overlaps' });
      await page.getByRole('button', { name: /^Mark as fine: Dash and Frost veil/ }).click();
      await panel.getByText('1 found', { exact: true }).waitFor();
      await page.getByRole('button', { name: /^Mark as fine: Sigils and Runes/ }).click();
      await page.getByRole('button', { name: /^Overlaps\s*,\s*2\s*marked as fine$/ }).waitFor();
      await panel.getByText('2 marked as fine', { exact: true }).waitFor();
    },
  },
];
