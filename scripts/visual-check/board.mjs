/**
 * The Board card's shots for scripts/visual-check.mjs (ONLY=board): a card
 * waiting for a By hand model with its export name line, its menu with the
 * By hand section first and the Move to submenu open (by hover and from the
 * keyboard), a right-click on a card, the header's tooltips and a game card
 * whose budget warnings run longer than the card, with the chip's tooltip.
 */

export async function boardShots({ open, shot }) {
  const board = await open('&scenario=configured', '#/board');
  await board.getByRole('button', { name: /^Character, v2/ }).waitFor();
  const lanterns = board.getByRole('button', { name: 'More for Stone lanterns' });
  const card = lanterns.locator('xpath=ancestor::div[contains(@class,"group/card")]');
  await card.getByRole('button', { name: 'Copy export name' }).waitFor();
  await board.mouse.move(0, 0);
  await shot(card, 'board-card-export-name');
  await board.context().grantPermissions(['clipboard-read', 'clipboard-write']);
  await card.getByRole('button', { name: 'Copy export name' }).click();
  await board.getByText('Export name copied').waitFor();
  await shot(board, 'board-card-export-name-copied');

  await lanterns.click();
  const menu = board.getByRole('menu', { name: 'Stone lanterns actions' });
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

  await board.getByRole('button', { name: /^Character, v2/ }).click({ button: 'right' });
  await board.getByRole('menu', { name: 'Character actions' }).waitFor();
  await shot(board, 'board-card-context-menu');
  await board.keyboard.press('Escape');

  await board.getByRole('button', { name: /^By hand/ }).hover();
  await board.getByRole('tooltip').waitFor();
  await shot(board, 'board-tooltip-by-hand');
  await board.getByRole('button', { name: /^3D queue/ }).hover();
  await board.getByRole('tooltip').filter({ hasText: '3D queue:' }).waitFor();
  await shot(board, 'board-tooltip-queue');
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
}
