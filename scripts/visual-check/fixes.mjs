/**
 * The dogfood fixes' shots for scripts/visual-check.mjs (ONLY=fixes): a
 * download's look card with its front render, its fit and the other ranked
 * sends, a rotate fix-up's degrees in the Inspector with the preview
 * numbers, the card menu's Add a model file item, the Budget tab's Game
 * folder column with a folder set, the Ship plan's Removed group for the
 * moved file, and Game export's drop maps switch.
 * Mock mode stands in for the Downloads folder and the game repo; nothing
 * real is read or written.
 */

const GAME = '&scenario=configured&game=';

async function lookShots({ open, shot }) {
  const look = await open('&scenario=configured&handoff=batch&arrival=look', '#/board');
  await look.getByText('Is this Lantern post?').first().waitFor();
  await look.getByRole('button', { name: 'Look' }).click();
  const dialog = look.getByRole('dialog', { name: 'New downloads' });
  await dialog.getByText('Also waiting:').waitFor();
  await look.mouse.move(0, 0);
  await shot(look, 'fixes-look-card');
  await shot(dialog, 'fixes-look-card-dialog');
  await look.context().close();
}

async function rotateShots({ open, shot }) {
  const page = await open('&scenario=configured', '#/inspector', { width: 1440, height: 1700 });
  await page.getByRole('heading', { name: 'Scene comparison' }).waitFor();
  const details = page.getByRole('complementary', { name: 'Model details' });
  await details.getByRole('button', { name: /^Elsheria/ }).click();
  const panel = details.getByRole('region', { name: 'Real size and fix-ups' });
  await panel.getByText(/^Measured/).waitFor();
  await panel.getByRole('button', { name: 'Rotate', exact: true }).click();
  const degrees = panel.getByLabel('Degrees about x');
  await degrees.fill('-24.5');
  await degrees.press('Enter');
  await panel.getByRole('button', { name: 'Preview the numbers' }).click();
  await panel.getByRole('table', { name: 'Before and after the fix-ups' }).waitFor();
  await panel.scrollIntoViewIfNeeded();
  await page.mouse.move(0, 0);
  await shot(panel, 'fixes-rotate-fixup');
  await page.context().close();
}

async function cardMenuShots({ open, shot }) {
  const board = await open('&scenario=configured', '#/board');
  await board.getByRole('button', { name: /^Elsheria, v2/ }).waitFor();
  await board.getByRole('button', { name: 'More for Elsheria' }).click();
  const item = board.getByRole('menuitem', { name: 'Add a model file as the next version...' });
  await item.hover();
  await shot(board, 'fixes-card-menu-model-file');
  await board.context().close();
}

async function gameShots({ open, shot }) {
  const page = await open(`${GAME}crypt`, '#/inspector', { width: 1440, height: 1100 });
  // The waiting Ship approval floats over the table; deny it out of the way.
  await page.getByRole('alertdialog').getByRole('button', { name: 'Deny' }).click();
  await page.getByRole('alertdialog').waitFor({ state: 'detached' });
  await page.getByRole('tab', { name: 'Budget' }).click();
  const field = page.getByRole('textbox', { name: /^Game folder of / }).first();
  await field.waitFor();
  await field.scrollIntoViewIfNeeded();
  await page.mouse.move(0, 0);
  await shot(page, 'fixes-budget-game-folder');
  await field.fill('props/lights');
  await field.press('Enter');
  await page.waitForTimeout(300);
  await shot(page, 'fixes-budget-game-folder-set');

  // The plan loads when Ship opens, so the moved file shows as removed.
  await page.evaluate(() => {
    window.location.hash = '#/pipeline';
  });
  await page.getByRole('button', { name: 'What Ship changes' }).click();
  await page.getByRole('heading', { name: 'Ship to Crypt game' }).waitFor();
  const removed = page.getByText('its asset moved folder').first();
  await removed.waitFor();
  await removed.scrollIntoViewIfNeeded();
  await page.mouse.move(0, 0);
  await shot(page, 'fixes-ship-removed');
  await page.context().close();

  // The crypt run waits for its Ship approval, which locks the settings.
  const runner = await open(`${GAME}maze`, '#/pipeline');
  const exportPanel = runner.getByRole('region', { name: 'Game export' });
  await exportPanel.waitFor();
  // The box itself is hidden for the custom switch; its label takes the pointer.
  await exportPanel.getByText('Drop normal and metal maps', { exact: true }).click();
  await exportPanel
    .getByRole('switch', { name: 'Drop normal and metal maps', checked: true })
    .waitFor({
      state: 'attached',
    });
  await exportPanel.scrollIntoViewIfNeeded();
  await runner.mouse.move(0, 0);
  await shot(exportPanel, 'fixes-game-export-drop-maps');
  await runner.context().close();
}

export async function fixesShots(tools) {
  await lookShots(tools);
  await rotateShots(tools);
  await cardMenuShots(tools);
  await gameShots(tools);
}
