/**
 * Removing a whole project for scripts/visual-check.mjs (ONLY=projects): a
 * new empty project's Board, whose Next step strip leaves out the Inbox
 * models other projects sent, the switcher with Remove and Removed
 * projects, the confirm that names the project, the refusal while a send
 * of Frostveil is not over, and Removed projects holding the one removed.
 * Each at full and minimum size. Nothing real is moved: the mock's trash
 * lives in memory.
 */

const SMALL = { width: 1120, height: 720 };

const switcher = page => page.locator('[aria-haspopup=menu]').first();

async function newEmptyProject(open, viewport, name) {
  const page = await open('&scenario=configured', '#/board', viewport);
  await switcher(page).click();
  await page.getByRole('menuitem', { name: 'New project' }).click();
  const dialog = page.getByRole('dialog', { name: 'New project' });
  await dialog.waitFor();
  await page.getByLabel('Project name').fill(name);
  await page.getByRole('button', { name: 'Create project' }).click();
  await dialog.waitFor({ state: 'detached' });
  await page.getByRole('button', { name: 'Hide tips' }).click();
  return page;
}

export async function projectsShots({ open, shot }) {
  for (const [viewport, suffix] of [
    [undefined, ''],
    [SMALL, '-small'],
  ]) {
    const name = 'Lantern study';
    const page = await newEmptyProject(open, viewport, name);
    const strip = page.getByRole('region', { name: 'Next step' });
    await strip.waitFor();
    await page.waitForTimeout(300);
    await shot(page, `project-empty-strip${suffix}`);

    // Frostveil has a send that is not over: Remove is refused, with why.
    await switcher(page).click();
    await page.getByRole('menuitemradio', { name: /^Frostveil/ }).click();
    await page
      .getByRole('button', { name: /^Elsheria/ })
      .first()
      .waitFor();
    await switcher(page).click();
    await page.getByRole('menuitem', { name: 'Remove Frostveil' }).waitFor();
    await page.mouse.move(0, 0);
    await shot(page, `project-switcher-remove${suffix}`);
    await page.getByRole('menuitem', { name: 'Remove Frostveil' }).click();
    const busy = page.getByRole('dialog', { name: 'Remove Frostveil?' });
    await busy.waitFor();
    await busy.getByRole('button', { name: 'Remove project' }).click();
    await busy.getByRole('alert').waitFor();
    await shot(page, `project-remove-refused${suffix}`);
    await busy.getByRole('button', { name: 'Cancel' }).click();

    // The empty one goes: its confirm, then Removed projects with it.
    await switcher(page).click();
    await page.getByRole('menuitemradio', { name: new RegExp(`^${name}`) }).click();
    await strip.waitFor();
    await switcher(page).click();
    await page.getByRole('menuitem', { name: `Remove ${name}` }).click();
    const ask = page.getByRole('dialog', { name: `Remove ${name}?` });
    await ask.waitFor();
    await page.waitForTimeout(300);
    await shot(page, `project-remove-confirm${suffix}`);
    await ask.getByRole('button', { name: 'Remove project' }).click();
    await ask.waitFor({ state: 'detached' });
    await page.getByText(`${name} is in the workspace trash`).waitFor();
    await shot(page, `project-removed-notice${suffix}`);
    await switcher(page).click();
    await page.getByRole('menuitem', { name: 'Removed projects' }).click();
    const list = page.getByRole('dialog', { name: 'Removed projects' });
    await list.getByRole('button', { name: `Restore ${name}` }).waitFor();
    await page.waitForTimeout(300);
    await shot(page, `project-removed-list${suffix}`);
    await page.context().close();
  }
}
