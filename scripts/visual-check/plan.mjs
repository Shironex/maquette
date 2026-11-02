/**
 * The Plan view's shots for scripts/visual-check.mjs (ONLY=plan): the list
 * of Frostveil's canvases with the New canvas dialog and a rename, Remove's
 * question, Combat ideas open with notes, pictures, a link and edges, a
 * node selected and a note being written, the Add a link dialog, a new
 * empty canvas, and the list and canvas at the minimum window size. Mock
 * mode's `?plans=demo` serves them.
 */

const SMALL = { width: 1120, height: 720 };

/** Opens the Plan view on Frostveil's demo canvases. */
async function plans(open, viewport) {
  const page = await open('&scenario=configured&plans=demo', '#/plan', viewport);
  await page.getByRole('region', { name: 'All canvases' }).waitFor();
  return page;
}

/** Opens Combat ideas and waits for its pictures. */
async function combat(page) {
  await page.getByRole('button', { name: 'Combat ideas', exact: true }).click();
  await page.getByRole('main', { name: 'Combat ideas canvas' }).waitFor();
  await page.getByRole('img', { name: 'Veil gate at night' }).waitFor();
  await page.waitForTimeout(600);
}

export async function planShots({ open, shot }) {
  const list = await plans(open);
  await shot(list, 'plan-list');

  await list.getByRole('button', { name: 'New canvas' }).first().click();
  await list.getByRole('textbox', { name: 'Name' }).fill('Lantern festival ideas');
  await shot(list, 'plan-new');
  await list.getByRole('button', { name: 'Cancel' }).click();

  await list.getByRole('button', { name: 'Rename Boss arena' }).click();
  await list.getByRole('textbox', { name: 'New name for Boss arena' }).fill('Warden arena');
  await shot(list, 'plan-rename');
  await list.keyboard.press('Escape');

  await list.getByRole('button', { name: 'Remove Opening story' }).click();
  await list.getByRole('dialog', { name: 'Remove Opening story?' }).waitFor();
  await shot(list, 'plan-remove');
  await list.getByRole('button', { name: 'Cancel' }).click();

  await combat(list);
  await shot(list, 'plan-canvas');

  await list.getByRole('group', { name: 'Note: Frost veil' }).click();
  await list.waitForTimeout(300);
  await shot(list, 'plan-selected');

  // A double click on the empty canvas writes a note there.
  const pane = await list.locator('.react-flow__pane').boundingBox();
  await list.mouse.dblclick(pane.x + pane.width * 0.5, pane.y + pane.height * 0.82);
  await list.getByRole('textbox', { name: 'Note text' }).fill('The lake cracks under the boss');
  await shot(list, 'plan-writing');
  await list.keyboard.press('Escape');

  await list.getByRole('button', { name: /^Link/ }).click();
  await list.getByRole('textbox', { name: 'Address' }).fill('example.com/frost-shaders');
  await list.getByRole('textbox', { name: 'Title' }).fill('Frost shaders that read at night');
  await shot(list, 'plan-link');
  await list.getByRole('button', { name: 'Add link' }).click();
  await list.getByRole('group', { name: 'Link: Frost shaders that read at night' }).waitFor();
  await list.getByText('saved', { exact: false }).waitFor();
  await shot(list, 'plan-added');

  await list.getByRole('button', { name: 'Plan, back to all canvases' }).click();
  await list.getByRole('button', { name: 'New canvas' }).first().click();
  await list.getByRole('textbox', { name: 'Name' }).fill('Lantern festival ideas');
  await list.getByRole('button', { name: 'Create' }).click();
  await list.getByRole('region', { name: 'This canvas is empty' }).waitFor();
  await shot(list, 'plan-empty');
  await list.context().close();

  const small = await plans(open, SMALL);
  await shot(small, 'plan-list-small');
  await combat(small);
  await shot(small, 'plan-canvas-small');
  await small.context().close();
}
