/**
 * The canvas presets' shots for the Plan view (ONLY=plan): the empty list
 * with its preset picker, the New canvas dialog with Story picked, and the
 * Story and Game mechanics canvases just made from their starter notes, at
 * full and at the minimum window size. Frostveil is a website project in
 * mock mode, so its Story starts worded for a visitor.
 */

const SMALL = { width: 1120, height: 720 };

/** Picks a preset card as a person does, by clicking it (the radio itself is hidden). */
const pick = (scope, name) =>
  scope
    .locator('label')
    .filter({ hasText: new RegExp(`^${name}`) })
    .click();

/** Makes a canvas from `preset` through New canvas and waits for its notes. */
async function make(page, name, preset, firstTitle) {
  await page.getByRole('button', { name: 'New canvas' }).first().click();
  const dialog = page.getByRole('dialog', { name: 'New canvas' });
  await dialog.getByRole('textbox', { name: 'Name' }).fill(name);
  await pick(dialog, preset);
  await dialog.getByRole('button', { name: 'Create' }).click();
  await page.getByRole('main', { name: `${name} canvas` }).waitFor();
  await page.getByRole('group', { name: `Note: ${firstTitle}` }).waitFor();
  await page.waitForTimeout(600);
}

export async function presetShots({ open, shot }) {
  for (const [viewport, suffix] of [
    [undefined, ''],
    [SMALL, '-small'],
  ]) {
    const empty = await open('&scenario=configured&plans=none', '#/plan', viewport);
    await empty.getByRole('radiogroup', { name: 'Start from' }).waitFor();
    await pick(empty, 'Game mechanics');
    await shot(empty, `plan-presets-empty${suffix}`);
    await empty.context().close();

    const page = await open('&scenario=configured&plans=demo', '#/plan', viewport);
    await page.getByRole('region', { name: 'All canvases' }).waitFor();
    await page.getByRole('button', { name: 'New canvas' }).first().click();
    const dialog = page.getByRole('dialog', { name: 'New canvas' });
    await dialog.getByRole('textbox', { name: 'Name' }).fill('Tour story');
    await pick(dialog, 'Story');
    await shot(page, `plan-presets-new${suffix}`);
    await dialog.getByRole('button', { name: 'Cancel' }).click();

    await make(page, 'Tour story', 'Story', 'Beat 1: First view');
    await shot(page, `plan-presets-story${suffix}`);
    await page.getByRole('button', { name: 'Plan, back to all canvases' }).click();
    await make(page, 'Combat systems', 'Game mechanics', 'System: Core action');
    await shot(page, `plan-presets-mechanics${suffix}`);
    await page.getByRole('button', { name: 'Plan, back to all canvases' }).click();
    await page.getByRole('region', { name: 'All canvases' }).waitFor();
    await page.waitForTimeout(300);
    await shot(page, `plan-presets-list${suffix}`);
    await page.context().close();
  }
}
