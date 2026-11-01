/**
 * The minimum window's shots for scripts/visual-check.mjs (ONLY=minsize):
 * at 1120 by 720 the Models header, the Inspector's Budget tab, the Board with
 * a card waiting for its model and the status bar, the By hand checklist
 * with the waiting batches first and the quick view kept inside the window,
 * which still closes with Escape after a preview mode is picked.
 */

const SMALL = { width: 1120, height: 720 };

export async function minSizeShots({ open, shot }) {
  const library = await open('&scenario=configured&library=demo', '#/library', SMALL);
  await library
    .getByRole('button', { name: /^Inspect 3D model of/ })
    .first()
    .waitFor();
  await shot(library, 'min-library');
  await library
    .getByRole('button', { name: /^Inspect 3D model of/ })
    .first()
    .click();
  const quick = library.getByRole('dialog', { name: 'Crystal lantern' });
  await quick.waitFor();
  await library.waitForTimeout(800);
  await shot(library, 'min-quick-view');
  await quick.getByRole('button', { name: 'Wireframe' }).click();
  await library.keyboard.press('Escape');
  await quick.waitFor({ state: 'detached' });

  const budget = await open('&scenario=configured&game=maze', '#/inspector', SMALL);
  await budget.getByRole('tab', { name: 'Budget' }).click();
  await budget.getByRole('columnheader', { name: 'Game folder' }).waitFor();
  await shot(budget, 'min-budget');

  const board = await open('&scenario=configured&handoff=batch&conceptMs=60000', '#/board', SMALL);
  await board.getByRole('button', { name: /^Elsheria, v2/ }).waitFor();
  await shot(board, 'min-board');
  await board.getByRole('button', { name: /^Sends\s*\d+/ }).click();
  await board.getByRole('tab', { name: /^By hand/ }).click();
  await board.getByRole('dialog', { name: 'By hand checklist' }).waitFor();
  await shot(board, 'min-checklist');
}
