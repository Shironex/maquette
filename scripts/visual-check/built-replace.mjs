/**
 * A built asset in the place of another model, for scripts/visual-check.mjs
 * (ONLY=built): the built card's menu, the sheet before and after a model is
 * picked (both models side by side, then what happens to every setting), a
 * pair that cannot be swapped yet, the Board once replaced, the replaced
 * card's menu and the sheet that brings the old model back, and the swap as
 * the person who approves the next Ship sees it: in the website Ship view
 * and on an agent's Ship approval.
 */

export async function builtReplaceShots({ open, shot }) {
  const board = await open('&scenario=configured&built=swap', '#/board');
  await board.getByRole('button', { name: /^Lantern rebuilt, v1/ }).waitFor();
  await board.getByRole('button', { name: 'More for Lantern rebuilt' }).click();
  await board.getByRole('menu', { name: 'Lantern rebuilt actions' }).waitFor();
  await shot(board, 'built-replace-menu');
  await board.getByRole('menuitem', { name: 'Use in place of a model...' }).click();
  // The title follows the built asset picked, so the sheet is found by its ending.
  const sheet = board.getByRole('dialog', { name: /in place of a model$/ });
  await sheet.waitFor();
  await board.mouse.move(0, 0);
  await shot(board, 'built-replace-pick');
  const takes = sheet.getByRole('combobox', { name: /Takes the place of/ });
  await takes.selectOption({ label: 'Crystal lantern' });
  await sheet.getByRole('table').waitFor();
  await shot(board, 'built-replace-report');
  // The end of the report: what stays, and that nothing is aligned.
  await sheet.getByText(/^Nothing is aligned/).scrollIntoViewIfNeeded();
  await shot(board, 'built-replace-report-settings');
  // A built asset whose script changed since its build cannot take a place.
  await sheet
    .getByRole('combobox', { name: /Built asset/ })
    .selectOption({ label: 'Newtons cradle' });
  await sheet.getByText('This pair cannot be swapped yet').waitFor();
  await shot(board, 'built-replace-refused');
  await sheet
    .getByRole('combobox', { name: /Built asset/ })
    .selectOption({ label: 'Lantern rebuilt' });
  await sheet.getByRole('table').waitFor();
  await sheet.getByRole('button', { name: 'Replace' }).click();
  await sheet.waitFor({ state: 'detached' });
  await board.getByText("Lantern rebuilt is in Crystal lantern's place").waitFor();
  await board.mouse.move(0, 0);
  await shot(board, 'built-replaced-board');
  await board.getByRole('button', { name: 'More for Crystal lantern' }).click();
  await board.getByRole('menu', { name: 'Crystal lantern actions' }).waitFor();
  await shot(board, 'built-replaced-menu');
  await board.getByRole('menuitem', { name: 'Bring back the old model...' }).click();
  const undo = board.getByRole('dialog', { name: 'Bring back the old model of Crystal lantern' });
  await undo.getByRole('region', { name: 'Current again' }).waitFor();
  await board.mouse.move(0, 0);
  await shot(board, 'built-replace-undo');
  await undo.getByRole('button', { name: 'Bring it back' }).click();
  await undo.waitFor({ state: 'detached' });
  await board.getByText('Crystal lantern is back on v2').waitFor();
  await board.mouse.move(0, 0);
  await shot(board, 'built-replace-undone');
  await board.context().close();

  // The swap where the next Ship is looked at: the website Ship view.
  const ship = await open('&scenario=configured&built=swapped', '#/pipeline');
  await ship.getByRole('button', { name: 'Ship settings' }).click();
  await ship.getByRole('heading', { name: "Ship to Frostveil's site" }).waitFor();
  await ship
    .getByRole('region', { name: 'Built models that replace another model in this Ship' })
    .waitFor();
  await ship.mouse.move(0, 0);
  await shot(ship, 'built-replace-ship-view');
  await ship.context().close();

  // And on the approval an agent's run raises before Ship.
  const prompt = await open('&scenario=configured&agents=active&built=swapped', '#/board');
  const card = prompt.getByRole('alertdialog', { name: /^Ship Frostveil, run/ });
  await card.waitFor();
  const review = card.getByRole('button', { name: /^Review:/ });
  if (await review.count()) await review.click();
  await card.getByText(/Crystal lantern is a built model now/).waitFor();
  await shot(prompt, 'built-replace-ship-approval');
  await shot(card, 'built-replace-ship-approval-card');
  await prompt.context().close();
}
