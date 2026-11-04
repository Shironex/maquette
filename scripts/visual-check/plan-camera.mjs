/**
 * The camera plan's shots for the Plan view (ONLY=plan): the pinned camera
 * plan in the list, its canvas built from Frostveil's layout stops with
 * their poster renders, a camera agent's proposal beside the main line,
 * the Apply review after taking the proposal (stops that move, come and
 * go, the moved stops leaving and coming back in the diff), the refusal
 * when layout.json changes while the review is open, the plan behind
 * layout.json with Compare, and the canvas and review at the minimum
 * window size. `mock_camera_agent` and `mock_layout_outside` make the
 * outside writes.
 */

const SMALL = { width: 1120, height: 720 };

export const mock = (page, cmd, args) =>
  page.evaluate(([c, a]) => window.__TAURI_INTERNALS__.invoke(c, a), [cmd, args]);

export const FROSTVEIL = { project: 'frostveil' };

export async function openCamera(open, viewport, extra = '') {
  const page = await open(`&scenario=configured&plans=demo${extra}`, '#/plan', viewport);
  await page.getByRole('region', { name: 'Pinned' }).waitFor();
  await page.getByRole('region', { name: 'Pinned' }).getByRole('button').click();
  await page.getByRole('main', { name: 'Camera plan canvas' }).waitFor();
  await page.getByRole('region', { name: 'Order after Apply' }).getByText('Arrival').waitFor();
  await page.waitForTimeout(700);
  return page;
}

/** The proposal taken as the main line, then Apply's review open. */
export async function review(page) {
  await mock(page, 'mock_camera_agent', FROSTVEIL);
  await page.getByRole('group', { name: 'Proposal by Camera agent' }).waitFor();
  await page.waitForTimeout(500);
  await page.getByRole('button', { name: 'Use this order' }).click();
  await page.getByText('saved', { exact: false }).waitFor();
  await page.getByRole('region', { name: 'Order after Apply' }).getByText('new stop').waitFor();
  await page.waitForTimeout(500);
  await page.getByRole('button', { name: 'Apply to layout.json' }).click();
  await page.getByRole('dialog', { name: 'Apply to layout.json' }).waitFor();
  await page.waitForTimeout(400);
}

export async function cameraShots({ open, shot }) {
  const list = await open('&scenario=configured&plans=demo', '#/plan');
  await list.getByRole('region', { name: 'Pinned' }).waitFor();
  await shot(list, 'plan-camera-pinned');
  await list.context().close();

  const page = await openCamera(open);
  await shot(page, 'plan-camera');
  await mock(page, 'mock_camera_agent', FROSTVEIL);
  await page.getByRole('group', { name: 'Proposal by Camera agent' }).waitFor();
  await page.getByRole('button', { name: 'Fit the canvas in view' }).click();
  await page.waitForTimeout(600);
  await shot(page, 'plan-camera-proposal');
  await page.getByRole('button', { name: 'Use this order' }).click();
  await page.getByText('saved', { exact: false }).waitFor();
  await page.getByRole('region', { name: 'Order after Apply' }).getByText('new stop').waitFor();
  await page.waitForTimeout(500);
  await page.getByRole('button', { name: 'Apply to layout.json' }).click();
  await page.getByRole('dialog', { name: 'Apply to layout.json' }).waitFor();
  await page.waitForTimeout(400);
  await shot(page, 'plan-camera-apply');

  // layout.json changes while the review is open: Apply refuses and says to review again.
  await mock(page, 'mock_layout_outside', FROSTVEIL);
  await page.getByRole('button', { name: 'Apply', exact: true }).click();
  await page.getByText('Nothing was applied', { exact: true }).waitFor();
  await page.waitForTimeout(300);
  await shot(page, 'plan-camera-refused');
  await page.getByRole('button', { name: 'Cancel' }).click();
  await page.getByText('layout.json changed since this plan was based on it').waitFor();
  await page.waitForTimeout(400);
  await shot(page, 'plan-camera-stale');
  await page.getByRole('button', { name: 'Compare' }).click();
  await page.getByRole('dialog', { name: 'layout.json changed' }).waitFor();
  await page.waitForTimeout(400);
  await shot(page, 'plan-camera-compare');
  await page.context().close();

  const small = await openCamera(open, SMALL);
  await shot(small, 'plan-camera-small');
  await review(small);
  await shot(small, 'plan-camera-apply-small');
  await small.context().close();
}
