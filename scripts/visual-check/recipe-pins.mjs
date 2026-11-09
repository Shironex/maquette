/**
 * The recipe pins' shots for scripts/visual-check.mjs (ONLY=motion): the
 * joined demo recipe with a new foot pin on its own Pin lane and the pin
 * settings open, with the preview's check under them, and the lantern
 * recipe with a hand pinned to its blank. Mock mode's `?library=demo`
 * recipes serve them.
 */

const DEMO = '&scenario=configured&motion=demo&library=demo';

/** Opens recipe `name` in the editor, paused at `share` of its frames. */
async function editRecipe(open, name, share) {
  const page = await open(DEMO, '#/motion');
  await page.getByRole('region', { name: 'Models that move' }).waitFor();
  await page.getByRole('tab', { name: 'Recipes' }).click();
  await page
    .getByRole('list', { name: 'Project recipes' })
    .getByRole('listitem')
    .filter({ hasText: name })
    .getByRole('button', { name: 'Edit' })
    .click();
  await page.getByRole('img', { name: /rough preview$/ }).waitFor();
  await page.waitForTimeout(2500);
  await page.getByRole('button', { name: 'Pause' }).click();
  const playhead = page.getByRole('slider', { name: 'Playhead' });
  const frames = Number(await playhead.getAttribute('max'));
  await playhead.fill(String(Math.round(frames * share)));
  return page;
}

/** Adds a pin from the Changes picker; its settings open. */
async function addPin(page) {
  await page.getByLabel('Kind of change').selectOption('pin');
  await page.getByRole('button', { name: 'Add change' }).click();
  await page.getByRole('group', { name: 'Pin, left foot' }).waitFor();
}

export async function recipePinShots({ open, shot }) {
  const page = await editRecipe(open, 'Slash into block', 0.4);
  await addPin(page);
  const check = page.getByText(/^The foot can(not)? stay on its spot/);
  await check.waitFor();
  await check.scrollIntoViewIfNeeded();
  await page.waitForTimeout(800);
  await shot(page, 'motion-recipe-pin');
  await page.context().close();

  const hand = await editRecipe(open, 'Reach for the lantern', 0.5);
  await addPin(hand);
  const pin = hand.getByRole('group', { name: 'Pin, left foot' });
  await pin.getByRole('button', { name: 'L hand' }).click();
  const settings = hand.getByRole('group', { name: 'Pin, left hand' });
  await settings.getByLabel('Stays at').selectOption('lantern');
  await settings.getByText('Hand pins take a blank').scrollIntoViewIfNeeded();
  await hand.waitForTimeout(800);
  await shot(hand, 'motion-recipe-pin-hand');
  await hand.context().close();
}
