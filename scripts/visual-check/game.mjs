/**
 * Game projects' shots for scripts/visual-check.mjs (ONLY=game): New
 * project with the Game target and with a website's check line, the Import
 * models list, the game pipeline with its Game export panel, the per-folder
 * budget, the Ship diff with its approval and imported originals, the game
 * project's rail, Motion with a rigged hero and 187 clips, the Board after
 * Render previews, the Budget tab with a rigged model, and the Inbox
 * checking a sent model against its folder's budget with the top bar's
 * budget tooltip.
 * Mock mode's `?game=` projects stand in for a game repo; nothing is read
 * from or written to a real one.
 */

const GAME = '&scenario=configured&game=';

/** Opens New project from the project switcher. */
async function newProject(open) {
  const page = await open('&scenario=configured', '#/board');
  await page.locator('[aria-haspopup=menu]').first().click();
  await page.getByRole('menuitem', { name: 'New project' }).click();
  await page.getByRole('dialog', { name: 'New project' }).waitFor();
  return page;
}

export async function gameShots({ open, shot }) {
  const game = await newProject(open);
  await game.getByLabel('Project name').fill('Crypt game');
  await game.getByRole('button', { name: /^Game/ }).click();
  const repo = game.getByLabel('Game repo');
  await repo.fill('~/Documents/Projects/crypt-game');
  // Tab commits the path as Enter does, without submitting the form.
  await repo.press('Tab');
  await game.getByText('CMake project found.').waitFor();
  await shot(game, 'game-new-project');
  await game.getByRole('button', { name: /^Level kit, \.obj/ }).click();
  await game.getByText(/kit\/ at 5k triangles/).waitFor();
  await game.getByText(/Model format below wins/).waitFor({ state: 'detached' });
  await shot(game, 'game-new-project-template');
  await game.context().close();

  const site = await newProject(open);
  await site.getByLabel('Project name').fill('Frostveil');
  const siteRepo = site.getByLabel('Target site repo');
  await siteRepo.fill('~/Documents/Projects/frostveil-site');
  await siteRepo.press('Tab');
  await site.getByText('Git repo found.').waitFor();
  await shot(site, 'game-website-check-line');
  await site.context().close();

  const fresh = await open(`${GAME}import`, '#/board');
  await fresh.getByRole('button', { name: 'More ways to add' }).click();
  await fresh.getByRole('menuitem', { name: 'Import models' }).click();
  const sheet = fresh.getByRole('dialog', { name: /^Import models from / });
  await sheet.getByRole('button', { name: /^Import \d+ models$/ }).waitFor();
  await shot(fresh, 'game-import-list');
  await fresh.context().close();

  const run = await open(`${GAME}crypt`, '#/pipeline');
  await run.getByRole('heading', { name: 'Ship to Crypt game' }).waitFor();
  await run.getByText('3 new · 2 replaced · 0 removed').first().waitFor();
  await shot(run, 'game-ship-approval');
  await run.getByText('replaces the imported original').first().scrollIntoViewIfNeeded();
  await shot(run.getByRole('main'), 'game-ship-originals');
  await run.getByRole('button', { name: 'Back to the runner' }).click();
  await run.getByRole('region', { name: 'Game export' }).waitFor();
  await shot(run, 'game-pipeline');
  await run.context().close();

  const kit = await open(`${GAME}import`, '#/pipeline');
  await kit.getByRole('button', { name: 'Use a template' }).click();
  const templates = kit.getByRole('dialog', { name: 'Use a template' });
  await templates.getByRole('button', { name: /^Level kit, \.obj/ }).waitFor();
  await shot(kit, 'game-templates-empty');
  await templates.getByRole('button', { name: /^Level kit, \.obj/ }).click();
  await templates.getByText('What will change').waitFor();
  await shot(kit, 'game-templates-preview');
  await templates.getByRole('button', { name: 'Apply Level kit, .obj' }).click();
  await templates.waitFor({ state: 'detached' });
  await kit.getByRole('button', { name: '.obj + .mtl', pressed: true }).waitFor();
  await kit.getByText(/kit\/ 1024 px/).waitFor();
  await shot(kit, 'game-templates-applied');
  await kit.context().close();

  const budget = await open(`${GAME}maze`, '#/inspector');
  await budget.getByRole('tab', { name: 'Budget' }).click();
  await budget
    .getByRole('button', { name: /^Over budget/ })
    .first()
    .waitFor();
  await shot(budget, 'game-folder-budget');
  await budget.context().close();

  const motion = await open(`${GAME}hero`, '#/motion');
  await motion.getByRole('button', { name: /Heroine/ }).click();
  await motion.getByText('Clips · 187').waitFor();
  await motion.getByRole('meter', { name: /^Game budget/ }).waitFor();
  await shot(motion, 'game-motion-hero');
  await motion.getByRole('searchbox', { name: 'Search clips' }).fill('parry overhead');
  await motion.getByText(/^Clips · \d+ of 187$/).waitFor();
  await shot(motion, 'game-motion-search');
  await motion.context().close();

  const board = await open(`${GAME}crypt`, '#/board');
  const more = board.getByRole('button', { name: 'More board actions' });
  // This fixture's approval prompt sits over the row, so the keyboard opens the menu.
  await more.focus();
  await board.keyboard.press('Enter');
  await board.getByRole('menuitem', { name: /^Render \d+ previews?$/ }).click();
  await board.waitForTimeout(1500);
  await more.focus();
  await board.keyboard.press('Enter');
  await board.getByRole('menu', { name: 'More board actions' }).waitFor();
  await board.getByRole('menuitem', { name: /^Render/ }).waitFor({ state: 'detached' });
  await board.keyboard.press('Escape');
  await board.waitForLoadState('networkidle');
  await shot(board, 'game-board-previews');
  await board.context().close();

  const rigged = await open(`${GAME}hero`, '#/inspector');
  await rigged.getByRole('tab', { name: 'Budget' }).click();
  await rigged
    .getByRole('button', { name: /^Over budget/ })
    .first()
    .click();
  await rigged.getByText('By hand').first().waitFor();
  await shot(rigged, 'game-budget-rigged');
  await rigged.context().close();

  const rail = await open(`${GAME}crypt`, '#/board');
  await rail.getByText('Game preview').waitFor();
  await shot(rail, 'game-project-rail');
  await rail.context().close();

  // A model sent for a prop checks against the props budget in the Inbox,
  // the same budget the top bar counts against.
  const inbox = await open(`${GAME}crypt`, '#/inbox');
  // The waiting Ship approval floats over the checks; deny it out of the way.
  await inbox.getByRole('alertdialog').getByRole('button', { name: 'Deny' }).click();
  await inbox.getByRole('alertdialog').waitFor({ state: 'detached' });
  await inbox
    .getByRole('button', { name: /^Candle cluster/ })
    .first()
    .click();
  await inbox.getByText(/^Over the props budget, 12k tris\./).waitFor();
  await shot(inbox, 'game-inbox-folder-budget');
  await inbox.context().close();
  const top = await open(`${GAME}crypt`, '#/motion');
  const meter = top.getByRole('meter', { name: /^Game budget/ });
  await meter.hover();
  await top.getByRole('tooltip').waitFor();
  await shot(top, 'game-budget-tooltip');
  await top.context().close();
}
