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
    .getByRole('region', { name: '3D queue' })
    .getByRole('button', { name: 'Credits' })
    .click();
  await page.getByRole('dialog', { name: 'Credits' }).getByText('1,240').waitFor();
}

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
];
