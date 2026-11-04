/**
 * The sheets' shots for scripts/visual-check.mjs (ONLY=sheets): with an
 * agent's approval cards floating, the Plan view's Apply review, its
 * Compare after layout.json changed and the Export pack sheet keep every
 * button in their bottom bar clear of the cards, at 1440 by 1000 and at
 * the minimum window size. The camera panel's Apply to layout.json is
 * checked on the way.
 */
import { assertClear } from './floating.mjs';
import { FROSTVEIL, mock, openCamera } from './plan-camera.mjs';

const SIZES = [
  [{ width: 1440, height: 1000 }, ''],
  [{ width: 1120, height: 720 }, '-min'],
];

/** Every button in the open sheet's bottom bar, checked against the cards. */
async function assertFooterClear(page, sheet, what) {
  await page.getByRole('alertdialog').first().waitFor();
  const buttons = await sheet.locator('[data-sheet-footer] button').all();
  if (!buttons.length) throw new Error(`${what} has no footer buttons to check`);
  for (const button of buttons) {
    const name = (await button.textContent())?.trim();
    await assertClear(page, button, `${name} in ${what}`);
  }
}

/**
 * The camera agent's proposal taken, then Apply's review open. At the
 * minimum size the proposal sits in the panel's list behind the cards, as
 * any content may, so its button is pressed without a pointer.
 */
async function reviewUnderCards(page) {
  await mock(page, 'mock_camera_agent', FROSTVEIL);
  await page.getByRole('group', { name: 'Proposal by Camera agent' }).waitFor();
  await page.getByRole('button', { name: 'Use this order' }).dispatchEvent('click');
  await page.getByRole('region', { name: 'Order after Apply' }).getByText('new stop').waitFor();
  await page.waitForTimeout(500);
  await page.getByRole('button', { name: 'Apply to layout.json' }).click();
  await page.getByRole('dialog', { name: 'Apply to layout.json' }).waitFor();
  await page.waitForTimeout(400);
}

export async function sheetShots({ open, shot }) {
  for (const [viewport, suffix] of SIZES) {
    const plan = await openCamera(open, viewport, '&agents=batch');
    // The camera panel's own bar keeps its Apply clear too.
    await plan.getByRole('alertdialog').first().waitFor();
    const toLayout = plan.getByRole('button', { name: 'Apply to layout.json' });
    await assertClear(plan, toLayout, 'Apply to layout.json');
    await shot(plan, `sheet-camera-panel-cards${suffix}`);
    await reviewUnderCards(plan);
    const apply = plan.getByRole('dialog', { name: 'Apply to layout.json' });
    await assertFooterClear(plan, apply, 'the Apply review');
    await shot(plan, `sheet-apply-cards${suffix}`);
    await mock(plan, 'mock_layout_outside', FROSTVEIL);
    await plan.getByRole('button', { name: 'Apply', exact: true }).click();
    await plan.getByText('Nothing was applied', { exact: true }).waitFor();
    await plan.getByRole('button', { name: 'Cancel' }).click();
    await plan.getByRole('button', { name: 'Compare' }).click();
    const compare = plan.getByRole('dialog', { name: 'layout.json changed' });
    await compare.getByRole('button', { name: 'Rebase' }).waitFor();
    await assertFooterClear(plan, compare, 'Camera compare');
    await shot(plan, `sheet-compare-cards${suffix}`);
    await plan.context().close();

    const library = await open(
      '&scenario=configured&library=demo&agents=batch',
      '#/library',
      viewport
    );
    await library.getByRole('button', { name: 'Export pack', exact: true }).click();
    const pack = library.getByRole('dialog', { name: 'Export pack' });
    await pack.getByRole('button', { name: 'Export and check' }).waitFor();
    await assertFooterClear(library, pack, 'Export pack');
    await shot(library, `sheet-pack-cards${suffix}`);
    await library.context().close();
  }
}
