/**
 * The Inspector's compare menu for scripts/visual-check.mjs (ONLY=compare):
 * a project with 52 assets, the long menu capped to the window and scrolled
 * inside, the search narrowing it, and the menu opening upwards low in a
 * short window.
 */

export async function compareShots({ open, shot }) {
  const page = await open('&scenario=configured&inspect=many', '#/inspector');
  const trigger = page.getByRole('button', { name: /vs/ }).first();
  await trigger.waitFor();
  await trigger.click();
  const search = page.getByRole('combobox', { name: 'Search compare' });
  await search.waitFor();
  await shot(page, 'compare-menu-long');
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('ArrowDown');
  await page.getByRole('listbox', { name: 'Compare' }).evaluate(el => el.scrollBy(0, 600));
  await shot(page, 'compare-menu-scrolled');
  await search.fill('LANT');
  await shot(page, 'compare-menu-search');
  await page.keyboard.press('Escape');
  await page.keyboard.press('Escape');
  await page.getByRole('combobox', { name: 'Search compare' }).waitFor({ state: 'detached' });
  await page.context().close();

  const short = await open('&scenario=configured&inspect=many', '#/inspector', {
    width: 1100,
    height: 520,
  });
  await short.getByRole('button', { name: /vs/ }).first().click();
  await short.getByRole('combobox', { name: 'Search compare' }).waitFor();
  await shot(short, 'compare-menu-short-window');
  await short.context().close();
}
