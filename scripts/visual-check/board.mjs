/**
 * The Board card's shots for scripts/visual-check.mjs (ONLY=board): a card
 * waiting for a By hand model with its export name line, its menu with the
 * By hand section first and the Move to submenu open (by hover and from the
 * keyboard), a right-click on a card, the header's controls with their menus
 * (website and game, Board and List, at the window's narrow widths) and a game card
 * whose budget warnings run longer than the card, with the chip's tooltip,
 * and the card menu in a short window: on the leftmost column's first card,
 * and on the bottom card of a column, where it flips up and stays inside.
 * Remove from project: the menu item with its tooltip, the confirm dialog
 * for one card with the By hand send it stops and for a selection with a
 * card a paid send is making, the notice with its Undo and Recently removed.
 */

/** The header row: its menus, and Board and List at the narrow widths the window allows. */
async function toolbarShots(page, shot, name) {
  const size = page.viewportSize();
  await page.getByRole('button', { name: 'More board actions' }).click();
  await page.getByRole('menu', { name: 'More board actions' }).waitFor();
  await shot(page, `board-toolbar-${name}-more`);
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: 'More ways to add' }).click();
  await page.getByRole('menu', { name: 'Add to the project' }).waitFor();
  await shot(page, `board-toolbar-${name}-add`);
  await page.keyboard.press('Escape');
  const layout = page.getByRole('group', { name: 'Board layout' });
  for (const width of [1100, 1120, 1440]) {
    await page.setViewportSize({ width, height: size.height });
    await shot(page, `board-toolbar-${name}-board-${width}`);
    await layout.getByRole('button', { name: 'List' }).click();
    await shot(page, `board-toolbar-${name}-list-${width}`);
    await layout.getByRole('button', { name: 'Board' }).click();
  }
  await page.setViewportSize(size);
}

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
  await board.getByRole('button', { name: 'More board actions' }).click();
  await board.getByRole('menuitem', { name: /^Removed/ }).click();
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

/**
 * At the 1120 by 720 minimum: the header's bright button on an empty and a
 * populated project, the ... menu, a normal, a rigged and a By hand card's
 * menu, the 40 px target of the dots, Export pack staying over the Board, and
 * several approval cards folded and opened.
 */
async function headerMenuShots({ open, shot }) {
  const HD = { width: 1120, height: 720 };
  const empty = await open('&scenario=configured&game=import', '#/board', HD);
  await empty.getByRole('heading', { name: 'Your first build in three steps' }).waitFor();
  await empty.mouse.move(0, 0);
  await shot(empty, 'header-empty');
  await empty.getByRole('button', { name: 'Hide tips' }).click();
  await empty.mouse.move(0, 0);
  await shot(empty, 'header-empty-tips-hidden');
  await empty.context().close();

  const page = await open('&scenario=configured', '#/board', HD);
  await page.getByRole('button', { name: /^Elsheria, v2/ }).waitFor();
  await page.mouse.move(0, 0);
  await shot(page, 'header-populated');
  await page.getByRole('button', { name: 'More board actions' }).click();
  await page.getByRole('group', { name: 'Make' }).waitFor();
  await page.mouse.move(0, 0);
  await shot(page, 'header-more-menu');
  await page.keyboard.press('Escape');
  for (const [name, file] of [
    ['Veil platform', 'normal'],
    ['Elsheria', 'rigged'],
    ['Guardian statues', 'handoff'],
  ]) {
    const dots = page.getByRole('button', { name: `More for ${name}` });
    if (file === 'normal') {
      // The corners of the 40 px target still hit the button, the card did not move.
      const box = await dots.boundingBox();
      const hits = await dots.evaluate(
        (el, { x, y, width, height }) =>
          [
            [x + width / 2 - 19, y + height / 2 - 19],
            [x + width / 2 + 19, y + height / 2 + 19],
          ].every(([px, py]) => {
            const at = document.elementFromPoint(px, py);
            return at === el || el.contains(at);
          }),
        box
      );
      if (!hits) throw new Error('the card dots do not have their 40 px target');
    }
    await dots.click();
    await page.getByRole('menu', { name: `${name} actions` }).waitFor();
    await page.mouse.move(0, 0);
    await shot(page, `card-menu-${file}`);
    await page.keyboard.press('Escape');
  }
  await page.getByRole('button', { name: 'More board actions' }).click();
  await page.getByRole('menuitem', { name: 'Export pack' }).click();
  await page
    .getByRole('dialog', { name: 'Export pack' })
    .getByText(/made ready/)
    .waitFor();
  if (!page.url().endsWith('#/board')) throw new Error(`Export pack left the Board: ${page.url()}`);
  await page.mouse.move(0, 0);
  await shot(page, 'header-pack-sheet');
  await page.context().close();

  const asked = await open('&scenario=configured&agents=many', '#/board', HD);
  const preview = asked.getByRole('alertdialog', {
    name: 'Run the dev command of frostveil-site?',
  });
  await preview.waitFor();
  await asked.mouse.move(0, 0);
  await shot(asked, 'approvals-collapsed');
  await asked.getByRole('button', { name: 'Review: Send 3 assets to Tripo?' }).click();
  await asked.mouse.move(0, 0);
  await shot(asked, 'approvals-expanded');
  await preview.getByRole('button', { name: /^Review/ }).click();
  await preview.getByRole('button', { name: 'Approve and run' }).scrollIntoViewIfNeeded();
  await asked.mouse.move(0, 0);
  await shot(asked, 'approvals-preview-expanded');
  await asked.context().close();
}

export async function boardShots({ open, shot }) {
  await headerMenuShots({ open, shot });
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

  // From the keyboard: End to Remove, two up to Move to, ArrowRight into the submenu.
  await lanterns.click();
  await menu.waitFor();
  await board.keyboard.press('End');
  await board.keyboard.press('ArrowUp');
  await board.keyboard.press('ArrowUp');
  await board.keyboard.press('ArrowRight');
  await board.getByRole('menu', { name: 'Move to' }).getByRole('menuitem').first().waitFor();
  await shot(board, 'board-card-menu-move-keys');
  await board.keyboard.press('Escape');
  await board.keyboard.press('Escape');

  await board.getByRole('button', { name: /^Elsheria, v2/ }).click({ button: 'right' });
  await board.getByRole('menu', { name: 'Elsheria actions' }).waitFor();
  await shot(board, 'board-card-context-menu');
  await board.keyboard.press('Escape');

  await board.getByRole('button', { name: /^Sends/ }).hover();
  await board.getByRole('tooltip').waitFor();
  await shot(board, 'board-tooltip-sends');
  await board.mouse.move(0, 0);
  await toolbarShots(board, shot, 'website');
  await removeShots(board, shot);
  await board.context().close();

  const fresh = await open('&scenario=configured&game=import', '#/board');
  await toolbarShots(fresh, shot, 'game');
  await fresh.context().close();

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
