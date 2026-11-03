/**
 * The Plan view's shots for scripts/visual-check.mjs (ONLY=plan): the list
 * of Frostveil's canvases with the New canvas dialog and a rename, Remove's
 * question, Combat ideas open with notes, pictures, a link and edges, a
 * node selected and a note being written, the Add a link dialog, a new
 * empty canvas, the list and canvas at the minimum window size, and the
 * question asked when the save on the way back fails. Then the asset links
 * on Boss: Frost Warden, read live (ready, to check, no model, a file
 * missing, a removed asset and a picked Lab direction), the removed one up
 * close, the Asset picker and a linked asset added, and the same canvas at
 * the minimum size. Then the history: an agent's run landing on Combat
 * ideas (its toast, the nodes it added and edited marked, the view panned
 * to them), the Changes panel at full and minimum size, the confirm when a
 * Keep would take earlier changes along, and the choice
 * when an edit from another computer meets an unsaved edit to the same
 * note. Then the presets (plan-presets.mjs). Mock mode's `?plans=demo` serves them (`?plans=locked`: the same,
 * every save failing); the mock only commands `mock_plan_agent` and
 * `mock_plan_outside` make the outside writes.
 */

import { cameraShots } from './plan-camera.mjs';
import { presetShots } from './plan-presets.mjs';

const SMALL = { width: 1120, height: 720 };

/** Opens the Plan view on Frostveil's demo canvases. */
async function plans(open, viewport, mode = 'demo') {
  const page = await open(`&scenario=configured&plans=${mode}`, '#/plan', viewport);
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

/** Opens Boss: Frost Warden and waits for the live read of its links. */
async function boss(page) {
  await page.getByRole('button', { name: 'Boss: Frost Warden', exact: true }).click();
  await page.getByRole('main', { name: 'Boss: Frost Warden canvas' }).waitFor();
  await page.getByRole('group', { name: 'Asset: Elsheria' }).waitFor();
  await page.getByRole('group', { name: 'Asset: Asset not found' }).waitFor();
  await page.waitForTimeout(600);
}

/** The asset links: live states, a removed asset, the picker and an added link. */
async function assetShots(open, shot) {
  const page = await plans(open);
  await boss(page);
  await shot(page, 'plan-assets');

  const gone = page.getByRole('group', { name: 'Asset: Asset not found' });
  await gone.click();
  for (let i = 0; i < 3; i += 1) await page.getByRole('button', { name: 'Zoom in' }).click();
  await page.waitForTimeout(400);
  const box = await gone.boundingBox();
  const pane = await page.locator('.react-flow__pane').boundingBox();
  // Bring the removed asset to the middle of the view.
  await page.mouse.move(box.x + box.width / 2, box.y - 30);
  await page.mouse.down();
  await page.mouse.move(pane.x + pane.width / 2, pane.y + pane.height / 2 - 60, { steps: 8 });
  await page.mouse.up();
  await page.waitForTimeout(400);
  await shot(page, 'plan-asset-missing');
  await page.getByRole('button', { name: 'Fit the canvas in view' }).click();

  await page.getByRole('button', { name: /^Asset/ }).click();
  const picker = page.getByRole('dialog', { name: 'Link assets' });
  await picker.waitFor();
  await picker.getByText('Crystal lantern').click();
  await picker.getByText('Veil gate kit').click();
  await page.waitForTimeout(300);
  await shot(page, 'plan-picker');
  await picker.getByRole('button', { name: 'Add 2' }).click();
  await page.getByRole('group', { name: 'Asset: Crystal lantern' }).waitFor();
  await page.getByText('saved', { exact: false }).waitFor();
  await page.waitForTimeout(600);
  await shot(page, 'plan-asset-added');
  await page.context().close();

  const small = await plans(open, SMALL);
  await boss(small);
  await shot(small, 'plan-assets-small');
  await small.context().close();
}

/** Runs a mock only command, as an agent's or another computer's write. */
const mock = (page, cmd, args) =>
  page.evaluate(([c, a]) => window.__TAURI_INTERNALS__.invoke(c, a), [cmd, args]);

const COMBAT = { project: 'frostveil', id: 'combat-ideas' };

/** An agent's run, the Changes panel, and an outside edit meeting an unsaved one. */
async function historyShots(open, shot) {
  for (const [viewport, suffix] of [
    [undefined, ''],
    [SMALL, '-small'],
  ]) {
    const page = await plans(open, viewport);
    await combat(page);
    await mock(page, 'mock_plan_agent', COMBAT);
    await page.getByText('Research agent changed the canvas').waitFor();
    await page.waitForTimeout(700);
    await shot(page, `plan-agent${suffix}`);
    await page.getByRole('button', { name: /^Changes/ }).click();
    await page.getByRole('complementary', { name: 'Changes' }).waitFor();
    await page.waitForTimeout(300);
    await shot(page, `plan-history${suffix}`);
    // A second agent edit of a person note in the same run: Keep per change and Keep run.
    await mock(page, 'mock_plan_agent', { ...COMBAT, edit: 'n-02' });
    await page.getByRole('button', { name: 'Keep run' }).waitFor();
    await page.waitForTimeout(300);
    await shot(page, `plan-keep${suffix}`);
    if (!suffix) {
      // A third edit of the same note: keeping the newest asks to keep the one under it too.
      await mock(page, 'mock_plan_agent', { ...COMBAT, edit: 'n-02' });
      const keep = page.getByRole('button', { name: /^Keep: Edited note: Rune combos/ });
      await keep.nth(1).waitFor();
      await keep.first().click();
      await page.getByRole('dialog', { name: 'Keep the earlier changes too?' }).waitFor();
      await page.waitForTimeout(300);
      await shot(page, 'plan-keep-confirm');
    }
    await page.context().close();
  }

  const page = await plans(open);
  await combat(page);
  await page.getByRole('group', { name: 'Note: Frost veil' }).dblclick();
  await page.getByRole('textbox', { name: 'Note text' }).fill('Hold Shift to slow time.');
  await page.keyboard.press('Control+Enter');
  await mock(page, 'mock_plan_outside', {
    ...COMBAT,
    node: 'n-01',
    text: 'Slow time while Dash is held, from the laptop.',
  });
  await page.getByRole('dialog', { name: 'The canvas changed outside' }).waitFor();
  await page.waitForTimeout(300);
  await shot(page, 'plan-outside');
  // Escape closes the question without choosing: the top bar says so and asks again.
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: 'Answer clash' }).waitFor();
  await page.waitForTimeout(300);
  await shot(page, 'plan-clash');
  await page.context().close();
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

  // Every save fails: a new note, then Back, asks before dropping it.
  const locked = await plans(open, undefined, 'locked');
  await combat(locked);
  await locked.getByRole('button', { name: /^Note/ }).click();
  await locked.getByRole('textbox', { name: 'Note text' }).fill('Kept until you choose');
  await locked.keyboard.press('Control+Enter');
  await locked.getByRole('button', { name: 'Plan, back to all canvases' }).click();
  await locked.getByRole('dialog', { name: 'The canvas was not saved' }).waitFor();
  await locked.waitForTimeout(300);
  await shot(locked, 'plan-unsaved');
  await locked.context().close();

  await assetShots(open, shot);
  await historyShots(open, shot);
  await cameraShots({ open, shot });
  await presetShots({ open, shot });
}
