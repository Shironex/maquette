/**
 * The canvas extras: an agent's removal proposal (the node's badge and the
 * ask in Changes), the Auto layout after agents switch turned off, the
 * Overlaps panel of a Game mechanics canvas with a pair marked as fine (then
 * both, where the button keeps a quiet count), and
 * starting a canvas from a node (the tool over the selected node, then the
 * new canvas). Each at full and minimum size. Mock mode's `?plans=systems`
 * serves Frostveil's demo canvases and Combat systems; the mock only
 * command `mock_plan_propose` makes the agent's ask.
 */

const SMALL = { width: 1120, height: 720 };
const COMBAT = { project: 'frostveil', id: 'combat-ideas' };

async function plans(open, viewport) {
  const page = await open('&scenario=configured&plans=systems', '#/plan', viewport);
  await page.getByRole('region', { name: 'All canvases' }).waitFor();
  return page;
}

async function canvas(page, name) {
  await page
    .getByRole('region', { name: 'All canvases' })
    .getByRole('button', { name, exact: true })
    .click();
  await page.getByRole('main', { name: `${name} canvas` }).waitFor();
  await page.waitForTimeout(600);
}

const mock = (page, cmd, args) =>
  page.evaluate(([c, a]) => window.__TAURI_INTERNALS__.invoke(c, a), [cmd, args]);

export async function extrasShots({ open, shot }) {
  for (const [viewport, suffix] of [
    [undefined, ''],
    [SMALL, '-small'],
  ]) {
    const page = await plans(open, viewport);
    await canvas(page, 'Combat ideas');
    await mock(page, 'mock_plan_propose', {
      ...COMBAT,
      nodes: ['n-06'],
      reason: 'The Frost veil card answers it now.',
    });
    await page.getByText('asked to remove').waitFor();
    await page.getByRole('button', { name: /^Changes/ }).click();
    await page.getByRole('button', { name: 'Keep it' }).waitFor();
    await page.waitForTimeout(300);
    await shot(page, `plan-ask${suffix}`);

    await page.getByRole('button', { name: /^Auto layout/ }).click();
    await page.getByRole('button', { name: /^Auto layout/, pressed: false }).waitFor();
    // A person's runs start folded: unfold the newest to show the switch's line.
    await page
      .getByRole('group', { name: 'You, 1 change' })
      .first()
      .getByRole('button', { name: 'Show the changes' })
      .click();
    await page.getByText('Turned auto layout after agents off').waitFor();
    await page.waitForTimeout(300);
    await shot(page, `plan-auto-off${suffix}`);

    await page.getByRole('button', { name: 'Close Changes' }).click();
    await page.getByRole('group', { name: 'Note: Frost veil' }).click();
    await page.getByRole('button', { name: 'Start a canvas from this' }).waitFor();
    await page.waitForTimeout(300);
    await shot(page, `plan-start-tool${suffix}`);
    await page.getByRole('button', { name: 'Start a canvas from this' }).click();
    await page.getByRole('main', { name: 'Frost veil canvas' }).waitFor();
    await page.waitForTimeout(600);
    await shot(page, `plan-started${suffix}`);

    await page.getByRole('button', { name: 'Plan, back to all canvases' }).click();
    await canvas(page, 'Combat systems');
    await page.getByRole('button', { name: /^Overlaps/ }).click();
    await page.getByRole('complementary', { name: 'Overlaps' }).waitFor();
    await page.waitForTimeout(300);
    await shot(page, `plan-overlaps${suffix}`);
    await page.getByRole('button', { name: /^Mark as fine: Dash and Frost veil/ }).click();
    await page
      .getByRole('complementary', { name: 'Overlaps' })
      .getByText('1 found', { exact: true })
      .waitFor();
    await page.waitForTimeout(400);
    await shot(page, `plan-overlap-fine${suffix}`);
    // Every overlap marked as fine: the button keeps a quiet count.
    await page.getByRole('button', { name: /^Mark as fine: Sigils and Runes/ }).click();
    await page.getByRole('button', { name: /^Overlaps\s*,\s*2\s*marked as fine$/ }).waitFor();
    await page
      .getByRole('complementary', { name: 'Overlaps' })
      .getByText('2 marked as fine', { exact: true })
      .waitFor();
    await page.waitForTimeout(400);
    await shot(page, `plan-overlap-all-fine${suffix}`);
    await page.context().close();
  }
}
