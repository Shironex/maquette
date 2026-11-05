/**
 * The Motion view's shots for scripts/visual-check.mjs (ONLY=motion): a
 * rigged figure playing a clip, a prop's states, a rig without clips, a
 * project where nothing moves, the scene over its budget with the clips in
 * it, the bone map, and the Recipes tab (empty, with the library's demo
 * recipes, and its New recipe base clip picker). Mock mode's `?motion=`
 * demo models, generated headless in Blender, load in the viewport.
 */

const DEMO = '&scenario=configured&motion=';

/** Opens the Motion view on a demo and waits for the list. */
async function motion(open, demo) {
  const page = await open(`${DEMO}${demo}`, '#/motion');
  await page.getByRole('region', { name: 'Models that move' }).waitFor();
  return page;
}

/** Waits for the viewport's model, then holds the clip at `frame`. */
async function holdAt(page, frame) {
  await page.getByRole('slider', { name: 'Playhead' }).waitFor();
  await page.waitForTimeout(1200);
  const playing = page.getByRole('button', { name: 'Pause' });
  if (await playing.isVisible()) await playing.click();
  await page.getByRole('button', { name: 'Go to In' }).click();
  for (let i = 0; i < frame; i += 1) await page.keyboard.press('.');
  await page.evaluate(() => document.activeElement?.blur());
  await page.waitForTimeout(400);
}

export async function motionShots({ open, shot }) {
  const main = await motion(open, 'demo');
  // The sets before this one leave their pages open, each with a live
  // canvas; on a full run they starve these two canvas pages of GPU time.
  for (const context of main.context().browser().contexts()) {
    if (context !== main.context()) await context.close();
  }
  await main.getByRole('button', { name: /^Walk loop/ }).click();
  await holdAt(main, 18);
  await main.getByText(/^Walk loop · frame 18$/).waitFor();
  await shot(main, 'motion-main');

  await main.getByRole('button', { name: /^Switch cabinet/ }).click();
  await main.getByText(/^Door open · end of the /).waitFor();
  // A new canvas: give it time to load the cabinet and frame it on a busy machine.
  await main.waitForTimeout(3500);
  await shot(main, 'motion-states');
  await main.context().close();

  // Map bones opens the retarget at its bone map step.
  const map = await motion(open, 'demo');
  await map.getByRole('button', { name: /^Clay figure/ }).click();
  await map.getByRole('button', { name: 'Map bones' }).click();
  await map.getByRole('dialog', { name: /^Add library clips to / }).waitFor();
  await map.getByRole('region', { name: 'Bone map' }).waitFor();
  await map.waitForTimeout(800);
  await shot(map, 'motion-retarget');
  await map.context().close();

  // The Recipes tab: empty without a library, then the demo recipes and the picker.
  const empty = await motion(open, 'demo');
  await empty.getByRole('tab', { name: 'Recipes' }).click();
  await empty.getByText('No recipes yet').waitFor();
  await shot(empty, 'motion-recipes-empty');
  await empty.context().close();

  const recipes = await motion(open, 'demo&library=demo');
  await recipes.getByRole('tab', { name: 'Recipes' }).click();
  await recipes.getByRole('list', { name: 'Project recipes' }).waitFor();
  await shot(recipes, 'motion-recipes');
  await recipes.getByRole('button', { name: 'New recipe' }).click();
  const picker = recipes.getByRole('dialog', { name: 'New recipe' });
  await picker.getByRole('list', { name: 'Library clips' }).getByRole('button').first().click();
  await picker.getByLabel('Recipe name').waitFor();
  await recipes.waitForTimeout(300);
  await shot(recipes, 'motion-recipe-picker');
  await recipes.context().close();

  const rest = await motion(open, 'rest');
  await rest.getByText('No clips on Clay figure yet').waitFor();
  await rest.waitForTimeout(1200);
  await shot(rest, 'motion-no-clips');
  await rest.context().close();

  const still = await motion(open, 'still');
  await still.getByText(/ could move$/).waitFor();
  await shot(still, 'motion-empty');
  await still.context().close();

  const over = await motion(open, 'over');
  await over.getByRole('button', { name: /^Elsheria\s*119 bones/ }).click();
  await over
    .getByRole('alert')
    .filter({ hasText: /over its .+ budget/ })
    .waitFor();
  await holdAt(over, 40);
  await shot(over, 'motion-over-budget');
  await over.context().close();
}
