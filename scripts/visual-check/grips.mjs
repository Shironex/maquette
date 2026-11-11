/**
 * The grip shots for scripts/visual-check.mjs (ONLY=grips): the Animations
 * library's Grips with a grip and its revisions on show; in the recipe
 * editor on Scythe charge, held, Add a grip with the scythe's staff grip
 * picked (the recipe holds r1, the library has r2), the grip's change with
 * its update offer and its lane; the save a grip sheet opened from the pose
 * keys on both hands; and an agent's proposed grip save as a card, then its
 * Review in the sheet. Mock mode's `?library=demo` grip library serves them.
 */

import { assertClear } from './floating.mjs';

const DEMO = '&scenario=configured&library=demo&motion=demo';

/** Opens Scythe charge, held in the recipe editor, paused. */
async function openScythe(open, extra = '') {
  const page = await open(`${DEMO}${extra}`, '#/motion');
  await page.getByRole('region', { name: 'Models that move' }).waitFor();
  await page.getByRole('tab', { name: 'Recipes' }).click();
  await page
    .getByRole('list', { name: 'Project recipes' })
    .getByRole('listitem')
    .filter({ hasText: 'Scythe charge, held' })
    .getByRole('button', { name: 'Edit' })
    .click();
  await page.getByRole('img', { name: /rough preview$/ }).waitFor();
  await page.waitForTimeout(2000);
  await page.getByRole('button', { name: 'Pause' }).click();
  return page;
}

export async function gripShots({ open, shot }) {
  const library = await open(DEMO, '#/animations');
  await library.getByRole('button', { name: 'Grips', exact: true }).click();
  await library
    .getByRole('list', { name: 'Library grips' })
    .getByRole('button', { name: 'Longsword, two hands' })
    .click();
  await library
    .getByRole('complementary', { name: 'Selected grip' })
    .getByRole('region', { name: 'What r3 holds' })
    .getByRole('list', { name: 'Bones' })
    .waitFor();
  await library.waitForTimeout(300);
  await shot(library, 'animations-grips');
  await library.context().close();

  const editor = await openScythe(open);
  await editor.getByLabel('Kind of change').selectOption('grip');
  await editor.getByRole('button', { name: 'Add a grip' }).click();
  const picker = editor.getByRole('region', { name: 'Add a grip' });
  await picker
    .getByRole('list', { name: 'Grips to add' })
    .getByRole('button', { name: 'Scythe, staff grip' })
    .click();
  await picker.getByText('Scythe, staff grip has r2').waitFor();
  await editor.waitForTimeout(300);
  await shot(editor, 'motion-recipe-grip-picker');
  await picker.getByRole('button', { name: 'Close' }).click();

  await editor.getByRole('button', { name: /^Grip l-2, frames/ }).click();
  const form = editor.getByRole('group', { name: 'Change l-2 settings' });
  await form.getByText('Scythe, staff grip has r2').waitFor();
  await editor.evaluate(() => document.activeElement?.blur());
  await editor.waitForTimeout(400);
  await shot(form, 'motion-recipe-grip-form');
  await shot(editor.getByRole('region', { name: 'Timeline' }), 'motion-recipe-grip-lane');

  await editor.getByRole('button', { name: 'Pose keys l-1' }).click();
  await editor.getByRole('slider', { name: 'Playhead' }).fill('30');
  await editor.getByRole('button', { name: 'Save a grip' }).click();
  const sheet = editor.getByRole('dialog', { name: 'Save a grip' });
  await sheet.getByText('What it holds').waitFor();
  await editor.waitForTimeout(400);
  await shot(editor, 'motion-recipe-grip-save');
  await editor.context().close();

  const asked = await open(`${DEMO}&agents=grip-save`, '#/motion');
  await asked.getByRole('region', { name: 'Models that move' }).waitFor();
  const card = asked.getByRole('alertdialog', { name: 'Save a grip: Scythe, staff grip r3' });
  await card.waitFor();
  await asked.waitForTimeout(400);
  await shot(asked, 'motion-recipe-grip-approval');
  await card.getByRole('button', { name: 'Review', exact: true }).click();
  const review = asked.getByRole('dialog', { name: 'Save a grip' });
  await review.getByText('What changes against r2').waitFor();
  await asked.waitForTimeout(400);
  await assertClear(asked, review.getByRole('button', { name: 'Save as r3' }), 'Save as r3');
  await assertClear(asked, review.getByRole('button', { name: 'Cancel' }), 'Cancel');
  await shot(asked, 'motion-recipe-grip-review');
  await asked.context().close();
}
