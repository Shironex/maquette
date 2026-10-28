/**
 * The Animations view's shots for scripts/visual-check.mjs (ONLY=animations):
 * the grid with the demo clips and one playing on its own skeleton, a pack
 * picked in the list layout, the empty library, Import clips with packs, a
 * duplicate and files left out, Save clips to library from a model in
 * Motion, the four Use on model steps from Motion's Add from library, and
 * the review after Pin the feet. Mock mode's `?library=demo` clips serve them; every clip plays
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

  const use = await open(`${DEMO}&motion=demo`, '#/motion');
  await use.getByRole('region', { name: 'Models that move' }).waitFor();
  await use.getByRole('button', { name: 'Add clip' }).click();
  await use.getByRole('menuitem', { name: 'Add from library' }).click();
  const steps = use.getByRole('dialog', { name: /^Add library clips to / });
  for (const name of [
    'Sword And Shield Run',
    'Sword And Shield Slash',
    'Sword And Shield Crouch',
    'Standing Dodge Left',
  ]) {
    await steps.getByText(name, { exact: true }).click();
  }
  await steps.getByText('Chosen · 4').waitFor();
  await shot(use, 'retarget-choose');
  // The bone map opens first only when the rig needs a look; open it from
  // the steps either way.
  const nav = steps.getByRole('navigation', { name: 'Steps' });
  await steps.getByRole('button', { name: 'Next: bone map' }).click();
  await steps.getByRole('region', { name: /^(Bone map|Clip settings)$/ }).waitFor();
  await nav.getByRole('button', { name: /Bone map/ }).click();
  await steps.getByRole('region', { name: 'Bone map' }).waitFor();
  await shot(use, 'retarget-map');
  await nav.getByRole('button', { name: /Clip settings/ }).click();
  await steps.getByRole('region', { name: 'Clip settings' }).waitFor();
  await steps.getByRole('button', { name: 'Standing Dodge Left' }).click();
  await shot(use, 'retarget-settings');
  await steps.getByRole('button', { name: 'Retarget 4 clips' }).click();
  await use.getByRole('dialog', { name: /^Review the retarget on / }).waitFor();
  await use.waitForTimeout(2500);
  await shot(use, 'retarget-review');
  await use.getByRole('button', { name: 'Pin the feet and bake again' }).click();
  await use.getByRole('group', { name: 'Feet pinned' }).waitFor();
  await use.waitForTimeout(2500);
  await shot(use, 'retarget-review-pinned');
  await use.context().close();
}
