/**
 * The Library view's shots for scripts/visual-check.mjs (ONLY=library): the
 * grid with the demo entries, a search with a tag picked, the empty library,
 * Add to library from a Board card's menu, a model used in the open project,
 * a Board card with an update waiting and the Update dialog's plan. Mock
 * mode's `?library=demo` entries and project origins serve them.
 */

const DEMO = '&scenario=configured&library=demo';

export async function libraryShots({ open, shot }) {
  const grid = await open(DEMO, '#/library');
  await grid.getByText('8 models · most used first').waitFor();
  await grid
    .getByRole('complementary', { name: 'Selected model' })
    .getByText('Versions · 3')
    .waitFor();
  await shot(grid, 'library-grid');

  await grid
    .getByRole('region', { name: 'Filters' })
    .getByRole('button', { name: /^props/ })
    .click();
  await grid.getByRole('searchbox', { name: 'Find a model' }).fill('street');
  await grid.getByText('1 of 8 models · most used first').waitFor();
  await shot(grid, 'library-search-filter');

  await grid
    .getByRole('region', { name: 'Filters' })
    .getByRole('button', { name: /^All models/ })
    .click();
  await grid.getByRole('searchbox', { name: 'Find a model' }).fill('');
  await grid
    .getByRole('region', { name: 'Models' })
    .getByRole('button', { name: 'Stone lantern' })
    .click();
  await grid.getByRole('button', { name: 'Add, no credits' }).click();
  await grid.getByText('Added Stone lantern to Moon Island, no credits').waitFor();
  await shot(grid, 'library-use-in-project');
  await grid.context().close();

  const empty = await open('&scenario=configured', '#/library');
  await empty.getByText('The library is empty').waitFor();
  await shot(empty, 'library-empty');
  await empty.context().close();

  const board = await open(DEMO, '#/board');
  await board.getByRole('button', { name: 'Update to v3' }).waitFor();
  await shot(board, 'library-update-board');

  await board.getByRole('button', { name: 'More for Island' }).click();
  await board.getByRole('menuitem', { name: 'Add to library...' }).click();
  const add = board.getByRole('dialog', { name: 'Add Island to the library' });
  await add.getByLabel('Tags').fill('scenes, night');
  await shot(board, 'library-add');
  await add.getByRole('button', { name: 'Cancel' }).click();

  await board.getByRole('button', { name: 'Update to v3' }).click();
  const update = board.getByRole('dialog', { name: 'Update Character to library v3' });
  await update.getByText('Decimate to 60k').waitFor();
  await shot(board, 'library-update-dialog');
  await board.context().close();
}
