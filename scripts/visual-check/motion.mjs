/**
 * The Motion view's shots for scripts/visual-check.mjs (ONLY=motion): a
 * rigged figure playing a clip, a prop's states, a rig without clips, a
 * project where nothing moves, the scene over its budget with the clips in
 * it, the bone map, and the Recipes tab (empty, with the library's demo
 * recipes and a finished bake, its New recipe base clip picker, a recipe
 * made on the rest pose open in the editor, Apply
 * baking a recipe then showing its numbers, and the review of a finished
 * bake with pass and warn numbers, also with an agent's approval card
 * floating clear of its bar, and the recipe library's sheets, see
 * recipe-library.mjs). Mock mode's `?motion=`
 * demo models, generated headless in Blender, load in the viewport.
 */

import { assertClear } from './floating.mjs';
import { recipeLibraryShots } from './recipe-library.mjs';

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

/** Opens the lantern recipe's finished bake in the review, both pictures playing. */
async function openReview(open, demo) {
  const page = await motion(open, demo);
  await page.getByRole('tab', { name: 'Recipes' }).click();
  await page
    .getByRole('list', { name: 'Project recipes' })
    .getByRole('listitem')
    .filter({ hasText: 'Reach for the lantern' })
    .getByRole('button', { name: 'Review' })
    .click();
  await page.getByRole('list', { name: 'Review numbers' }).waitFor();
  await page.getByText('Foot slide').first().waitFor();
  // Both canvases load their models and the base clip's bone map.
  await page.waitForTimeout(3000);
  return page;
}

/** The review of a finished bake: pass and warn numbers, then an agent's card clear of the bar. */
async function recipeReviewShots(open, shot) {
  const review = await openReview(open, 'demo&library=demo');
  await review.getByRole('button', { name: 'Pause' }).click();
  await shot(review, 'motion-recipe-review');
  await review.context().close();

  const asked = await openReview(open, 'demo&library=demo&agents=recipe');
  const card = asked.getByRole('alertdialog', { name: 'Add Clay figure v3?' });
  await card.waitFor();
  await asked.getByRole('button', { name: 'Pause' }).click();
  await asked.waitForTimeout(400);
  await assertClear(asked, asked.getByRole('button', { name: 'Add as v3' }), 'Add as v3');
  await assertClear(asked, asked.getByRole('button', { name: 'Close', exact: true }), 'Close');
  await assertClear(
    asked,
    asked.getByText('A warn asks for a look', { exact: false }),
    'the warn note'
  );
  await shot(asked, 'motion-recipe-review-approval');
  await asked.context().close();
}

/** The recipe editor: changes, the preview held at the reach's end, a blank, an error, leaving, a gone prop. */
async function recipeEditorShots(open, shot) {
  const page = await motion(open, 'demo&library=demo');
  await page.getByRole('tab', { name: 'Recipes' }).click();
  const list = page.getByRole('list', { name: 'Project recipes' });
  await list
    .getByRole('listitem')
    .filter({ hasText: 'Reach for the lantern' })
    .getByRole('button', { name: 'Edit' })
    .click();
  await page.getByRole('img', { name: /rough preview$/ }).waitFor();
  await page.waitForTimeout(2500);
  await page.getByRole('button', { name: 'Pause' }).click();
  const playhead = page.getByRole('slider', { name: 'Playhead' });
  const frames = Number(await playhead.getAttribute('max'));
  await playhead.fill(String(Math.round(frames * 0.33)));
  const changes = page.getByRole('list', { name: 'Recipe changes' });
  await changes.getByRole('button', { name: /^Reach/ }).click();
  await page.waitForTimeout(600);
  await shot(page, 'motion-recipe-editor');

  await changes.getByRole('button', { name: /^Reach/ }).click();
  await page
    .getByRole('list', { name: 'Recipe blanks' })
    .getByRole('button', { name: /^The prop the hand reaches/ })
    .click();
  await page.getByRole('group', { name: 'Blank lantern settings' }).waitFor();
  await page.waitForTimeout(300);
  await shot(page, 'motion-recipe-blank');

  await page
    .getByRole('list', { name: 'Recipe blanks' })
    .getByRole('button', { name: /^The prop the hand reaches/ })
    .click();
  await changes.getByRole('button', { name: /^Reach/ }).click();
  await page.getByLabel('Target').selectOption('');
  await page.getByRole('alert').filter({ hasText: 'A reach needs a target' }).waitFor();
  await page.waitForTimeout(300);
  await shot(page, 'motion-recipe-error');

  await page.getByRole('button', { name: 'Recipes', exact: true }).click();
  await page.getByRole('dialog', { name: 'Save the recipe first?' }).waitFor();
  await page.waitForTimeout(300);
  await shot(page, 'motion-recipe-leave');
  await page.getByRole('button', { name: 'Leave without saving' }).click();

  await list
    .getByRole('listitem')
    .filter({ hasText: 'Look around' })
    .getByRole('button', { name: 'Edit' })
    .click();
  await page.getByText('A prop is gone').waitFor();
  await page.waitForTimeout(2500);
  await shot(page, 'motion-recipe-warning');
  await page.context().close();
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
  // The rest pose as the base: no clip, and the rough preview holds the model's own pose.
  await picker.getByRole('button', { name: 'Rest pose' }).click();
  await picker.getByLabel('Recipe name').fill('Stand and wave');
  await recipes.waitForTimeout(300);
  await shot(recipes, 'motion-recipe-picker-rest');
  await picker.getByRole('button', { name: 'Make recipe' }).click();
  await picker.waitFor({ state: 'detached' });
  await recipes
    .getByRole('list', { name: 'Project recipes' })
    .getByRole('listitem')
    .filter({ hasText: 'Stand and wave' })
    .getByRole('button', { name: 'Edit' })
    .click();
  await recipes.getByRole('img', { name: /^The rest pose on .* rough preview$/ }).waitFor();
  await recipes.waitForTimeout(2500);
  await shot(recipes, 'motion-recipe-rest');
  await recipes.context().close();

  // Apply: the mock bake steps through its frames, then shows its numbers.
  const baking = await motion(open, 'demo&library=demo');
  await baking.getByRole('tab', { name: 'Recipes' }).click();
  const lantern = baking
    .getByRole('list', { name: 'Project recipes' })
    .getByRole('listitem')
    .filter({ hasText: 'Reach for the lantern' });
  await lantern.getByRole('button', { name: 'Apply' }).click();
  await lantern.getByRole('progressbar').waitFor();
  await baking.waitForTimeout(600);
  await shot(baking, 'motion-recipe-baking');
  await lantern.getByRole('button', { name: 'Review' }).waitFor({ timeout: 15000 });
  await baking.waitForTimeout(300);
  await shot(baking, 'motion-recipe-baked');
  await baking.context().close();

  await recipeReviewShots(open, shot);
  await recipeEditorShots(open, shot);
  await recipeLibraryShots({ open, shot });

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
