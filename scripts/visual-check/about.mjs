/**
 * Settings > About and the app updater, for scripts/visual-check.mjs
 * (ONLY=about): the section as it is until the first public release (the
 * updater inert), the header pill when an update is out (open and collapsed
 * rail), and About with that update: its notes, the download in progress
 * and Restart to update. Mock mode's `?updater=` states serve them.
 */

export async function aboutShots({ open, shot }) {
  const inert = await open('&scenario=configured', '#/settings');
  const about = inert.locator('#settings-about');
  await about.getByText('commit 4137061f2a, built 2026-10-27').waitFor();
  await inert
    .getByRole('navigation', { name: 'Settings sections' })
    .getByRole('button', { name: 'About' })
    .click();
  await about.scrollIntoViewIfNeeded();
  await shot(inert, 'about-inert-page');
  await shot(about, 'about-inert');
  await inert.context().close();

  const board = await open('&scenario=configured&updater=available', '#/board');
  const pill = board.getByRole('button', { name: 'Update 0.2.0' });
  await pill.waitFor();
  await shot(board, 'about-pill');
  await board.getByRole('button', { name: 'Collapse' }).click();
  await board.getByRole('button', { name: 'Update 0.2.0' }).waitFor();
  await shot(board, 'about-pill-collapsed');
  await board.getByRole('button', { name: /^Expand/ }).click();
  await board.getByRole('button', { name: 'Update 0.2.0' }).click();
  const section = board.locator('#settings-about');
  await section.getByRole('region', { name: 'Release notes for 0.2.0' }).waitFor();
  await section.scrollIntoViewIfNeeded();
  await shot(section, 'about-update-available');

  await section.getByRole('button', { name: 'Download 0.2.0' }).click();
  await section.getByRole('progressbar').waitFor();
  await shot(section, 'about-update-downloading');
  await section.getByRole('button', { name: 'Restart to update' }).waitFor();
  await shot(section, 'about-update-ready');
  // The browser never restarts: the mock refuses as for a running pipeline.
  await section.getByRole('button', { name: 'Restart to update' }).click();
  await section.getByText(/pipeline run or a build is still going/).waitFor();
  await shot(section, 'about-update-refused');
  await board.context().close();
}
