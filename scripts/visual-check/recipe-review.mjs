/**
 * The recipe review's round 2 shots for scripts/visual-check.mjs
 * (ONLY=recipereview): Slash into block baked and reviewed with its seam
 * pop, pose sheet and fixes, then Try it laying the best fix on the recipe
 * in the editor; an agent's drafts on Slash with a lean (`?agents=drafts`),
 * its banner, drafted change and blue keys; and the combat starters in the
 * Animations library, the left handed scythe charge on show.
 */

const DEMO = '&scenario=configured&library=demo&motion=demo';

/** Opens the Motion view's Recipes tab on the demo recipes. */
async function recipes(open, extra = '') {
  const page = await open(`${DEMO}${extra}`, '#/motion');
  await page.getByRole('region', { name: 'Models that move' }).waitFor();
  await page.getByRole('tab', { name: 'Recipes' }).click();
  await page.getByRole('list', { name: 'Project recipes' }).waitFor();
  return page;
}

const row = (page, name) =>
  page
    .getByRole('list', { name: 'Project recipes' })
    .getByRole('listitem')
    .filter({ hasText: name });

export async function recipeReviewShots({ open, shot }) {
  const page = await recipes(open);
  const joined = row(page, 'Slash into block');
  await joined.getByRole('button', { name: 'Apply' }).click();
  await joined.getByRole('button', { name: 'Review' }).waitFor({ timeout: 15000 });
  await joined.getByRole('button', { name: 'Review' }).click();
  await page.getByRole('list', { name: 'Review numbers' }).getByText('Seam pop').waitFor();
  const sheet = page.getByRole('region', { name: 'Pose sheet' });
  await sheet.locator('img[src^="data:"]').first().waitFor({ timeout: 20000 });
  await page.waitForTimeout(2500);
  await sheet.getByRole('button', { name: /seam 1/ }).click();
  await page.waitForTimeout(800);
  await shot(page, 'motion-recipe-review-seams');
  await shot(sheet, 'motion-recipe-review-sheet');
  await shot(page.getByRole('region', { name: 'Fixes' }), 'motion-recipe-review-fixes');
  await page
    .getByRole('region', { name: 'Fixes' })
    .getByRole('button', { name: 'Try it' })
    .first()
    .click();
  await page.getByRole('img', { name: /rough preview$/ }).waitFor();
  await page.waitForTimeout(1500);
  await page.getByRole('button', { name: 'Pause' }).click();
  await page.evaluate(() => document.activeElement?.blur());
  await page.waitForTimeout(300);
  await shot(page, 'motion-recipe-fix-tried');
  await page.context().close();

  const drafted = await recipes(open, '&agents=drafts');
  await row(drafted, 'Slash with a lean').getByRole('button', { name: 'Edit' }).click();
  await drafted.getByRole('img', { name: /rough preview$/ }).waitFor();
  await drafted.getByRole('status', { name: 'Agent drafts' }).waitFor();
  await drafted.waitForTimeout(2500);
  await drafted.getByRole('button', { name: 'Pause' }).click();
  await drafted.getByRole('button', { name: 'Pose keys l-1' }).click();
  await drafted.evaluate(() => document.activeElement?.blur());
  await drafted.waitForTimeout(600);
  await shot(drafted, 'motion-recipe-drafts');
  await shot(drafted.getByRole('region', { name: 'Timeline' }), 'motion-recipe-drafts-lanes');
  await drafted.context().close();

  const library = await open(DEMO, '#/animations');
  await library.getByRole('button', { name: 'Recipes', exact: true }).click();
  const starters = library.getByRole('region', { name: 'Starters' });
  await starters.getByRole('button', { name: 'Scythe charge, left handed, starter' }).click();
  const side = library.getByRole('complementary', { name: 'Selected starter' });
  await side.getByRole('list', { name: 'Clips' }).waitFor();
  await starters.getByRole('button', { name: 'Slash combo, starter' }).scrollIntoViewIfNeeded();
  await library.waitForTimeout(300);
  await shot(library, 'animations-recipe-starters-combat');
  await library.context().close();
}
