/**
 * The Paths editor's shots for scripts/visual-check.mjs (ONLY=paths):
 * Frostveil's rune line selected with its points, a point picked with the
 * gizmo on it, Add points with its hint, a stroke from Draw on surfaces, and a project without paths. Mock
 * mode's `?paths=demo` and `?paths=empty` serve them.
 */

const SLOW = 30_000;

/** Opens the Inspector's Paths tab and waits for the models to load. */
async function paths(open, demo) {
  const page = await open(`&scenario=configured&paths=${demo}`, '#/inspector');
  await page.getByRole('tab', { name: 'Paths' }).click();
  await page.getByRole('heading', { name: 'Scene paths' }).waitFor();
  await page
    .getByText(/Loading (\d+ models|the models)/)
    .waitFor({ state: 'detached', timeout: SLOW });
  await page.waitForTimeout(1500);
  return page;
}

export async function pathsShots({ open, shot }) {
  const main = await paths(open, 'demo');
  // Pages the sets before this one left open each keep a live canvas.
  for (const context of main.context().browser().contexts()) {
    if (context !== main.context()) await context.close();
  }
  await main.getByRole('heading', { name: 'Points' }).waitFor();
  await shot(main, 'paths-main');

  await main.getByRole('button', { name: 'Pick point 3' }).click();
  await main.getByText(/Rune line · 5 points · [\d.]+ m · point 3/).waitFor();
  await main.waitForTimeout(600);
  await shot(main, 'paths-point');

  await main.getByRole('button', { name: /^Add points/ }).click();
  await main.getByText(/^Click a model or the ground to add a point/).waitFor();
  await main.waitForTimeout(300);
  await shot(main, 'paths-add');

  // A drag over a model lays a run of points along it.
  await main.getByRole('button', { name: /^Draw on surfaces/ }).click();
  await main.getByText(/^Drag over a model to draw on it/).waitFor();
  const before = await main.getByRole('button', { name: /^Pick point/ }).count();
  const box = await main.getByRole('img', { name: 'Paths in the scene' }).boundingBox();
  const at = (x, y) => [box.x + box.width * x, box.y + box.height * y];
  await main.mouse.move(...at(0.485, 0.52));
  await main.mouse.down();
  for (let i = 1; i <= 24; i += 1)
    await main.mouse.move(...at(0.485 + i * 0.0005, 0.52 + i * 0.005));
  await main.mouse.up();
  await main.waitForTimeout(800);
  const after = await main.getByRole('button', { name: /^Pick point/ }).count();
  if (after <= before) throw new Error(`Draw on surfaces added no points (${before} to ${after})`);
  await shot(main, 'paths-surface');
  await main.context().close();

  const empty = await paths(open, 'empty');
  await empty.getByText('No paths yet', { exact: true }).first().waitFor();
  await shot(empty, 'paths-empty');
  await empty.context().close();
}
