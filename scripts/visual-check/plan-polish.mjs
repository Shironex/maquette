/**
 * The Plan view's empty states and minimum window size (ONLY=plan): the
 * recent canvases under Plan on the rail (one an agent changed since it was
 * last open, marked), an empty canvas, the camera plan with no stops and
 * with a stop that has no render yet, the Changes panel with nothing in it,
 * the Asset picker, the New canvas and Add a link dialogs, the map shown
 * again on a narrow canvas, the camera plan behind layout.json with
 * Compare, and a rebase refused because layout.json changed again while the
 * comparison was open. `mock_layout_bare` empties layout.json's stops.
 */

const SMALL = { width: 1120, height: 720 };
const FROSTVEIL = { project: 'frostveil' };

const mock = (page, cmd, args) =>
  page.evaluate(([c, a]) => window.__TAURI_INTERNALS__.invoke(c, a), [cmd, args]);

/** A canvas's button in the list's table (the rail lists the recent ones by name too). */
const inTable = (page, name) =>
  page.getByRole('region', { name: 'All canvases' }).getByRole('button', { name, exact: true });

async function plans(open, viewport) {
  const page = await open('&scenario=configured&plans=demo', '#/plan', viewport);
  await page.getByRole('region', { name: 'All canvases' }).waitFor();
  return page;
}

async function openIn(page, name) {
  await inTable(page, name).click();
  await page.getByRole('main', { name: `${name} canvas` }).waitFor();
  await page.waitForTimeout(500);
}

const back = page => page.getByRole('button', { name: 'Plan, back to all canvases' }).click();

/** The rail's recent canvases: Combat ideas open, Boss: Frost Warden changed by an agent since. */
async function railShot(open, shot, viewport, suffix) {
  const page = await plans(open, viewport);
  await openIn(page, 'Boss: Frost Warden');
  await back(page);
  await page.getByRole('region', { name: 'All canvases' }).waitFor();
  await page.waitForTimeout(300);
  await mock(page, 'mock_plan_agent', { ...FROSTVEIL, id: 'boss-frost-warden' });
  const recent = page.getByRole('list', { name: 'Recent canvases' });
  await recent.getByRole('button', { name: /^Boss: Frost Warden\s*, changed since/ }).waitFor();
  await recent.getByRole('button', { name: 'Combat ideas', exact: true }).click();
  await page.getByRole('main', { name: 'Combat ideas canvas' }).waitFor();
  await page.waitForTimeout(600);
  await shot(page, `plan-rail-recent${suffix}`);
  await page.context().close();
}

/** The camera plan with no stops, then one added: it has no render yet. */
async function bareCamera(open, shot, viewport, suffix) {
  const page = await open('&scenario=configured&plans=demo', '#/plan', viewport);
  const pinned = page.getByRole('region', { name: 'Pinned' });
  await pinned.waitFor();
  await mock(page, 'mock_layout_bare', FROSTVEIL);
  await pinned.getByRole('button').click();
  await page.getByRole('region', { name: 'No camera stops yet' }).waitFor();
  await page.getByText(/^No stops yet/).waitFor();
  await page.waitForTimeout(400);
  await shot(page, `plan-camera-empty${suffix}`);
  await page.getByRole('button', { name: 'Add stop' }).click();
  await page.getByText('saved', { exact: false }).waitFor();
  await page.getByText(/no render yet/).waitFor();
  await page.waitForTimeout(600);
  await shot(page, `plan-camera-norender${suffix}`);
  await page.context().close();
}

/** The camera plan behind layout.json, Compare, and a rebase refused for a newer change. */
async function staleCamera(open, shot) {
  const page = await open('&scenario=configured&plans=demo', '#/plan', SMALL);
  await page.getByRole('region', { name: 'Pinned' }).getByRole('button').click();
  await page.getByRole('region', { name: 'Order after Apply' }).getByText('Arrival').waitFor();
  await page.waitForTimeout(500);
  await mock(page, 'mock_layout_outside', FROSTVEIL);
  await page.getByText('layout.json changed since this plan was based on it').waitFor();
  await page.waitForTimeout(400);
  await shot(page, 'plan-camera-stale-small');
  await page.getByRole('button', { name: 'Compare' }).click();
  const compare = page.getByRole('dialog', { name: 'layout.json changed' });
  await compare.waitFor();
  await page.waitForTimeout(400);
  await shot(page, 'plan-camera-compare-small');
  await mock(page, 'mock_layout_outside', FROSTVEIL);
  await compare.getByRole('button', { name: 'Rebase' }).click();
  await compare.getByRole('button', { name: 'Review again' }).waitFor();
  await page.waitForTimeout(300);
  await shot(page, 'plan-camera-compare-refused');
  await page.context().close();
}

export async function polishShots({ open, shot }) {
  await railShot(open, shot, undefined, '');
  await railShot(open, shot, SMALL, '-small');
  await bareCamera(open, shot, undefined, '');
  await bareCamera(open, shot, SMALL, '-small');
  await staleCamera(open, shot);

  const small = await plans(open, SMALL);
  await small.getByRole('button', { name: 'New canvas' }).first().click();
  await small.getByRole('textbox', { name: 'Name' }).fill('Lantern festival ideas');
  await shot(small, 'plan-new-small');
  await small.getByRole('button', { name: 'Cancel' }).click();

  await openIn(small, 'Boss arena');
  await small.getByRole('region', { name: 'This canvas is empty' }).waitFor();
  await shot(small, 'plan-empty-small');
  await small.getByRole('button', { name: /^Changes/ }).click();
  await small.getByRole('complementary', { name: 'Changes' }).waitFor();
  await small.waitForTimeout(300);
  await shot(small, 'plan-changes-empty-small');
  await small.getByRole('button', { name: 'Close Changes' }).click();
  await small.getByRole('button', { name: 'Web page' }).click();
  await small.getByRole('dialog', { name: 'Add a link' }).waitFor();
  await shot(small, 'plan-link-small');
  await small.keyboard.press('Escape');

  await back(small);
  await openIn(small, 'Boss: Frost Warden');
  await small.getByRole('button', { name: 'Map' }).click();
  await small.getByRole('img', { name: 'Map of the canvas' }).waitFor();
  await small.waitForTimeout(300);
  await shot(small, 'plan-map-small');
  await small.getByRole('button', { name: /^Asset/ }).click();
  await small.getByRole('dialog', { name: 'Link assets' }).getByText('Frozen lake arena').waitFor();
  await small.waitForTimeout(300);
  await shot(small, 'plan-picker-small');
  await small.context().close();
}
