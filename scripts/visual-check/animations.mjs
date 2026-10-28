/**
 * The Animations view's shots for scripts/visual-check.mjs (ONLY=animations):
 * the grid with the demo clips and one playing on its own skeleton, a pack
 * picked in the list layout, the empty library, Import clips with packs, a
 * duplicate and files left out, and Save clips to library from a model in
 * Motion. Mock mode's `?library=demo` clips serve them; every clip plays
 * the bundled skeleton the core's reader wrote headless.
 */

const DEMO = '&scenario=configured&library=demo';

export async function animationsShots({ open, shot }) {
  const grid = await open(DEMO, '#/animations');
  await grid.getByText('16 clips · newest first').waitFor();
  await grid
    .getByRole('complementary', { name: 'Selected clip' })
    .getByRole('heading', { name: 'Standing Dodge Left' })
    .waitFor();
  await grid.waitForTimeout(1500);
  await shot(grid, 'animations-grid');

  await grid
    .getByRole('region', { name: 'Filters' })
    .getByRole('button', { name: /^Pro Sword and Shield Pack/ })
    .click();
  await grid.getByRole('button', { name: 'List' }).click();
  await grid.getByText('6 of 16 clips · newest first').waitFor();
  await grid
    .getByRole('region', { name: 'Clips' })
    .getByRole('button', { name: 'Sword And Shield Run' })
    .click();
  await grid.waitForTimeout(1200);
  await shot(grid, 'animations-list-pack');

  await grid.getByRole('button', { name: 'Import clips' }).click();
  const sheet = grid.getByRole('dialog', { name: 'Import clips' });
  await sheet.getByRole('button', { name: 'Add files' }).click();
  await sheet.getByText('Pro Sword and Shield Pack.zip', { exact: true }).waitFor();
  await shot(grid, 'animations-import');
  await sheet.getByLabel('Pack for Pro Magic Pack.zip').scrollIntoViewIfNeeded();
  await shot(grid, 'animations-import-into');
  await sheet.getByRole('button', { name: /^Import \d+ clips$/ }).click();
  await grid.getByText(/^Imported \d+ clips into the library$/).waitFor();
  await shot(grid, 'animations-imported');
  await grid.context().close();

  const empty = await open('&scenario=configured', '#/animations');
  await empty.getByText('No clips yet').waitFor();
  await shot(empty, 'animations-empty');
  await empty.context().close();

  const motion = await open(`${DEMO}&motion=demo`, '#/motion');
  await motion.getByRole('region', { name: 'Models that move' }).waitFor();
  await motion.getByRole('button', { name: 'Add clip' }).click();
  await motion.getByRole('menuitem', { name: 'Save clips to library' }).click();
  const save = motion.getByRole('dialog', { name: 'Save clips to the library' });
  await save.getByText(/rig, \d+ bones/).waitFor();
  await shot(motion, 'animations-save-clips');
  await motion.context().close();
}
