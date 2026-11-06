/**
 * The recipe library's shots for scripts/visual-check.mjs: the Animations
 * library's Recipes with an entry and its revisions on show, and the
 * built-in starters under it with one starter on show (ONLY=animations),
 * and in the Motion view's Recipes tab (ONLY=motion) the save to library
 * sheet with what changes, the import warning under the header, and an
 * agent's proposed save as a card over the tab, then its Review in the
 * sheet with the card floating clear of the sheet's bar. Mock mode's
 * `?library=demo` recipe library serves them.
 */

import { assertClear } from './floating.mjs';

const DEMO = '&scenario=configured&library=demo';

/** The Animations library's Recipes, the lantern recipe and its three revisions on show. */
export async function libraryRecipesShot({ open, shot }) {
  const page = await open(DEMO, '#/animations');
  await page.getByRole('button', { name: 'Recipes', exact: true }).click();
  await page
    .getByRole('list', { name: 'Library recipes' })
    .getByRole('button', { name: 'Reach for the lantern' })
    .click();
  const side = page.getByRole('complementary', { name: 'Selected recipe' });
  await side
    .getByRole('region', { name: 'What r3 holds' })
    .getByRole('list', { name: 'Changes' })
    .waitFor();
  await shot(page, 'animations-recipes');
  await page.context().close();
}

/** The Starters under the library's recipes, the rest pose wave on show. */
export async function starterRecipesShot({ open, shot }) {
  const page = await open(DEMO, '#/animations');
  await page.getByRole('button', { name: 'Recipes', exact: true }).click();
  const starters = page.getByRole('region', { name: 'Starters' });
  await starters.getByRole('button', { name: 'Wave hello, starter' }).click();
  const side = page.getByRole('complementary', { name: 'Selected starter' });
  await side.getByRole('list', { name: 'Changes' }).waitFor();
  await starters.scrollIntoViewIfNeeded();
  await page.waitForTimeout(300);
  await shot(page, 'animations-recipe-starters');
  await page.context().close();
}

/** Opens the Motion view's Recipes tab on the demo recipes. */
async function recipesTab(open, extra = '') {
  const page = await open(`${DEMO}&motion=demo${extra}`, '#/motion');
  await page.getByRole('region', { name: 'Models that move' }).waitFor();
  await page.getByRole('tab', { name: 'Recipes' }).click();
  await page.getByRole('list', { name: 'Project recipes' }).waitFor();
  return page;
}

export async function recipeLibraryShots({ open, shot }) {
  const save = await recipesTab(open);
  await save.getByText('Library has r3').waitFor();
  await save.getByRole('button', { name: 'More for Reach for the lantern' }).click();
  await save.getByRole('menuitem', { name: /^Save to library/ }).click();
  const sheet = save.getByRole('dialog', { name: 'Save to library' });
  await sheet.getByText('What changes against r3').waitFor();
  await save.waitForTimeout(300);
  await shot(save, 'motion-recipe-save');
  await save.context().close();

  const imported = await recipesTab(open);
  await imported.getByRole('button', { name: 'Import', exact: true }).click();
  await imported.getByRole('list', { name: 'Import notes' }).waitFor();
  await imported.waitForTimeout(300);
  await shot(imported, 'motion-recipe-import');
  await imported.context().close();

  const asked = await recipesTab(open, '&agents=recipe-save');
  const card = asked.getByRole('alertdialog', {
    name: 'Save Reach for the lantern to the recipe library?',
  });
  await card.waitFor();
  await asked.waitForTimeout(400);
  await shot(asked, 'motion-recipe-save-approval');
  await card.getByRole('button', { name: 'Review', exact: true }).click();
  const review = asked.getByRole('dialog', { name: 'Save to library' });
  await review.getByText('What changes against r3').waitFor();
  await asked.waitForTimeout(400);
  await assertClear(asked, review.getByRole('button', { name: 'Save as r4' }), 'Save as r4');
  await assertClear(asked, review.getByRole('button', { name: 'Cancel' }), 'Cancel');
  await shot(asked, 'motion-recipe-save-review');
  await asked.context().close();
}
