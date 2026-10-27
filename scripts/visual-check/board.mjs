/**
 * The Board card's shots for scripts/visual-check.mjs (ONLY=board): a card
 * waiting for a By hand model with its export name line, its menu with the
 * By hand section first and the Move to submenu open (by hover and from the
 * keyboard), a right-click on a card, the header's tooltips and a game card
 * whose budget warnings run longer than the card, with the chip's tooltip,
 * and the card menu in a short window: on the leftmost column's first card,
 * and on the bottom card of a column, where it flips up and stays inside.
 * Remove from project: the menu item with its tooltip, the confirm dialog
 * for one card with the By hand send it stops and for a selection with a
 * card a paid send is making, the notice with its Undo and Recently removed.
 */

async function removeShots(board, shot) {
  await board.getByRole('button', { name: 'More for Guardian statues' }).click();
  const remove = board.getByRole('menuitem', { name: 'Remove from project...' });
  await remove.hover();
  await board.getByRole('tooltip').filter({ hasText: 'workspace trash' }).waitFor();
  await shot(board, 'board-card-menu-remove');
  await remove.click();
  const dialog = board.getByRole('dialog', { name: 'Remove Guardian statues from the project?' });
  await dialog.waitFor();
  await board.mouse.move(0, 0);
  await shot(board, 'board-remove-dialog');
  await dialog.getByRole('button', { name: 'Remove' }).click();
  await board.getByText('Removed Guardian statues').waitFor();
  await shot(board, 'board-removed-notice');
  await board.getByRole('button', { name: /^Removed/ }).click();
  await board.getByRole('menu', { name: 'Recently removed' }).waitFor();
  await board.mouse.move(0, 0);
  await shot(board, 'board-recently-removed');
  await board.keyboard.press('Escape');

  await board.getByRole('checkbox', { name: 'Select Veil gate' }).click();
  await board.getByRole('checkbox', { name: 'Select Elsheria' }).click();
  await board.getByRole('checkbox', { name: 'Select Rune obelisk' }).click();
  await board
    .getByRole('toolbar', { name: 'Selected assets' })
    .getByRole('button', { name: 'Remove...' })
    .click();
  await board.getByRole('dialog', { name: 'Remove 3 assets from the project?' }).waitFor();
  await board.mouse.move(0, 0);
  await shot(board, 'board-remove-dialog-many');
  await board.keyboard.press('Escape');
}

export async function boardShots({ open, shot }) {
  const board = await open('&scenario=configured', '#/board');
  await board.getByRole('button', { name: /^Elsheria, v2/ }).waitFor();
  const lanterns = board.getByRole('button', { name: 'More for Guardian statues' });
  const card = lanterns.locator('xpath=ancestor::div[contains(@class,"group/card")]');
  await card.getByRole('button', { name: 'Copy export name' }).waitFor();
  await board.mouse.move(0, 0);
  await shot(card, 'board-card-export-name');
  await board.context().grantPermissions(['clipboard-read', 'clipboard-write']);
  await card.getByRole('button', { name: 'Copy export name' }).click();
  await board.getByText('Export name copied').waitFor();
  await shot(board, 'board-card-export-name-copied');

  await lanterns.click();
  const menu = board.getByRole('menu', { name: 'Guardian statues actions' });
  await menu.waitFor();
  await board.mouse.move(0, 0);
  await shot(board, 'board-card-menu-handoff');
  await menu.getByRole('menuitem', { name: 'Move to' }).hover();
  await board.getByRole('menu', { name: 'Move to' }).waitFor();
  await shot(board, 'board-card-menu-move');
  await board.keyboard.press('Escape');

  // From the keyboard: End to Move to, ArrowRight into the submenu.
  await lanterns.click();
  await menu.waitFor();
  await board.keyboard.press('End');
  await board.keyboard.press('ArrowRight');
  await board.getByRole('menu', { name: 'Move to' }).getByRole('menuitem').first().waitFor();
  await shot(board, 'board-card-menu-move-keys');
  await board.keyboard.press('Escape');
  await board.keyboard.press('Escape');

  await board.getByRole('button', { name: /^Elsheria, v2/ }).click({ button: 'right' });
  await board.getByRole('menu', { name: 'Elsheria actions' }).waitFor();
  await shot(board, 'board-card-context-menu');
  await board.keyboard.press('Escape');

  await board.getByRole('button', { name: /^By hand/ }).hover();
  await board.getByRole('tooltip').waitFor();
  await shot(board, 'board-tooltip-by-hand');
  await board.getByRole('button', { name: /^3D queue/ }).hover();
  await board.getByRole('tooltip').filter({ hasText: '3D queue:' }).waitFor();
  await shot(board, 'board-tooltip-queue');
  await removeShots(board, shot);
  await board.context().close();

  const game = await open('&scenario=configured&game=crypt', '#/board');
  const warning = game.getByText('Over 12k triangles, Texture over 1024 px').first();
  await warning.waitFor();
  const gate = warning.locator('xpath=ancestor::div[contains(@class,"group/card")]');
  await game.mouse.move(0, 0);
  await shot(gate, 'board-card-warnings');
  await warning.hover();
  await game.getByRole('tooltip').waitFor();
  await shot(game, 'board-card-warnings-tooltip');
  await game.context().close();

  const short = await open('&scenario=configured', '#/board', { width: 1280, height: 620 });
  await short.getByRole('button', { name: /^Elsheria, v2/ }).waitFor();
  const more = short.getByRole('button', { name: /^More for / });
  await more.first().click();
  await short.getByRole('menu', { name: / actions$/ }).waitFor();
  await short.mouse.move(0, 0);
  await shot(short, 'board-card-menu-leftmost');
  await short.keyboard.press('Escape');
  const last = more.last();
  await last.scrollIntoViewIfNeeded();
  await last.click();
  // The flipped menu can open under the pointer, so wait for the card's own.
  await short.getByRole('menu', { name: / actions$/ }).waitFor();
  await short.getByRole('menuitem', { name: 'Move to' }).hover();
  await short.getByRole('menu', { name: 'Move to' }).waitFor();
  await shot(short, 'board-card-menu-bottom');
  await short.keyboard.press('Escape');
  await short.context().close();
}
