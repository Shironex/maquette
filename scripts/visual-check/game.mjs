/**
 * Game projects' shots for scripts/visual-check.mjs (ONLY=game): New
 * project with the Game target and with a website's check line, the Import
 * models list, the game pipeline with its Game export panel, the per-folder
 * budget, the Ship diff with its approval, and the game project's rail.
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
  await game.context().close();

  const site = await newProject(open);
  await site.getByLabel('Project name').fill('Moon Island');
  const siteRepo = site.getByLabel('Target site repo');
  await siteRepo.fill('~/Documents/Projects/moon-island-site');
  await siteRepo.press('Tab');
  await site.getByText('Git repo found.').waitFor();
  await shot(site, 'game-website-check-line');
  await site.context().close();

  const fresh = await open(`${GAME}import`, '#/board');
  await fresh.getByRole('button', { name: 'Import models' }).first().click();
  const sheet = fresh.getByRole('dialog', { name: /^Import models from / });
  await sheet.getByRole('button', { name: /^Import \d+ models$/ }).waitFor();
  await shot(fresh, 'game-import-list');
  await fresh.context().close();

  const run = await open(`${GAME}crypt`, '#/pipeline');
  await run.getByRole('heading', { name: 'Ship to Crypt game' }).waitFor();
  await run.getByText('3 new · 3 replaced · 0 removed').first().waitFor();
  await shot(run, 'game-ship-approval');
  await run.getByRole('button', { name: 'Back to the runner' }).click();
  await run.getByRole('region', { name: 'Game export' }).waitFor();
  await shot(run, 'game-pipeline');
  await run.context().close();

  const budget = await open(`${GAME}maze`, '#/inspector');
  await budget.getByRole('tab', { name: 'Budget' }).click();
  await budget
    .getByRole('button', { name: /^Over budget/ })
    .first()
    .waitFor();
  await shot(budget, 'game-folder-budget');
  await budget.context().close();

  const rail = await open(`${GAME}crypt`, '#/board');
  await rail.getByText('Game preview').waitFor();
  await shot(rail, 'game-project-rail');
  await rail.context().close();
}
