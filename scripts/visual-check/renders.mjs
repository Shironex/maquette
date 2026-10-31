/**
 * Render styles for scripts/visual-check.mjs (ONLY=renders): the Render tab
 * in blueprint and clay, the turntable sheet in WebP and GIF, a GIF over the
 * target, the batch sheet from a Board selection, a batch running, nothing to
 * render with what goes wrong, and ffmpeg missing. Mock mode's `?render=`
 * serves them, with real headless renders of the crystal lantern.
 */

const BASE = '&scenario=configured';

async function renderTab(open, params) {
  const page = await open(`${BASE}${params}`, '#/inspector');
  await page.getByRole('tab', { name: 'Render' }).click();
  await page
    .getByRole('heading', { name: 'Render style' })
    .or(page.getByRole('heading', { name: 'Nothing to render yet' }))
    .first()
    .waitFor();
  const asset = page.getByLabel('Asset');
  if (await asset.count()) await asset.selectOption('crystal-lantern');
  await page.waitForTimeout(900);
  return page;
}

export async function rendersShots({ open, shot }) {
  const tab = await renderTab(open, '');
  await shot(tab, 'renders-blueprint');
  await tab.getByRole('button', { name: /^Clay/ }).click();
  await tab.waitForTimeout(900);
  await shot(tab, 'renders-clay');
  await tab.getByRole('button', { name: /^Turntable/ }).click();
  const sheet = tab.getByRole('dialog', { name: /^Turntable/ });
  await sheet.getByLabel('Moving area').waitFor();
  await shot(tab, 'renders-turntable-webp');
  await sheet.getByRole('button', { name: 'Animated GIF' }).click();
  await tab.waitForTimeout(900);
  await shot(tab, 'renders-turntable-gif');
  await tab.context().close();

  const big = await renderTab(open, '&render=big');
  await big.getByRole('button', { name: /^Turntable/ }).click();
  const bigSheet = big.getByRole('dialog', { name: /^Turntable/ });
  await bigSheet.getByRole('button', { name: 'Animated GIF' }).click();
  await bigSheet.getByRole('button', { name: 'Crop to the model' }).waitFor();
  await shot(big, 'renders-turntable-over');
  await big.context().close();

  const board = await open(BASE, '#/board');
  for (const name of ['Crystal lantern', 'Elsheria', 'Observatory arch']) {
    await board
      .getByRole('checkbox', { name: `Select ${name}` })
      .first()
      .click({ force: true });
  }
  await board
    .getByRole('toolbar', { name: 'Selected assets' })
    .getByRole('button', { name: 'Render...' })
    .click();
  const batch = board.getByRole('dialog', { name: /^Render/ });
  await batch.getByRole('region', { name: 'Apply to all' }).waitFor();
  await batch.getByRole('button', { name: 'Toon', exact: true }).click();
  await shot(board, 'renders-batch');
  await board.context().close();

  const running = await open(`${BASE}&render=running`, '#/board');
  await running
    .getByRole('checkbox', { name: 'Select Crystal lantern' })
    .first()
    .click({ force: true });
  await running
    .getByRole('toolbar', { name: 'Selected assets' })
    .getByRole('button', { name: 'Render...' })
    .click();
  await running.getByRole('progressbar', { name: 'Files written' }).waitFor();
  await shot(running, 'renders-batch-running');
  await running.context().close();

  const failures = await renderTab(open, '&render=failures');
  await shot(failures, 'renders-failures');
  await failures.getByLabel('Asset').selectOption({ label: 'Guardian statues · no model' });
  await failures.getByRole('heading', { name: 'Nothing to render yet' }).waitFor();
  await failures.waitForTimeout(300);
  await shot(failures, 'renders-empty');
  await failures.context().close();

  const noFfmpeg = await renderTab(open, '&render=no-ffmpeg');
  await noFfmpeg.getByRole('button', { name: /^Turntable/ }).click();
  const noSheet = noFfmpeg.getByRole('dialog', { name: /^Turntable/ });
  await noSheet.getByRole('button', { name: 'Animated GIF' }).click();
  await noSheet.getByRole('heading', { name: 'ffmpeg was not found' }).waitFor();
  await noSheet.getByLabel('Moving area').waitFor();
  await shot(noFfmpeg, 'renders-no-ffmpeg');
  await noFfmpeg.context().close();
}
