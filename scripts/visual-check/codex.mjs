/**
 * Codex usage for scripts/visual-check.mjs (ONLY=codex): the status bar's
 * Codex item in the Lab while plenty is left (with its reset times in the
 * tooltip), amber with the composer's warning before a large round, red
 * with the Lab's usage limit banner over the jobs it holds, Limit lifted,
 * retry now while the limit still holds and once it has lifted, a
 * Codex that cannot say, and the image queue with jobs waiting: Run next
 * on its header and on a hovered waiting row.
 */

export async function codexShots({ open, shot, hoverTip }) {
  const lab = '&scenario=configured&conceptMs=60000';

  const ok = await open(`${lab}&codex=ok`, '#/lab');
  const item = ok.getByRole('button', { name: 'Codex 16% (5h) · 3% (week)' });
  await item.waitFor();
  await hoverTip(ok, item, 'codex-usage-ok');
  await ok.context().close();

  const low = await open(`${lab}&codex=low`, '#/lab');
  await low.getByRole('button', { name: 'Codex 86% (5h) · 41% (week)' }).waitFor();
  await low.getByRole('heading', { name: 'Frozen lake arena' }).waitFor();
  const note = low.getByRole('note');
  await note.waitFor();
  await note.scrollIntoViewIfNeeded();
  await shot(low, 'codex-usage-low');
  await low.context().close();

  const limit = await open(`${lab}&codex=limit&lab=usage`, '#/lab');
  const red = limit.getByRole('button', { name: 'Codex 100% (5h) · 44% (week)' });
  await red.waitFor();
  await limit.getByText('Codex usage limit reached').waitFor();
  await shot(limit, 'codex-usage-limit');
  await hoverTip(limit, red, 'codex-usage-limit-tip');
  await limit.getByRole('button', { name: 'Limit lifted, retry now' }).click();
  await limit.getByText(/^The usage limit still holds until about/).waitFor();
  await shot(limit, 'codex-usage-still-limited');
  await limit.context().close();

  const lifted = await open(`${lab}&codex=lifted&lab=usage`, '#/lab');
  await lifted.getByText('Codex usage limit reached').waitFor();
  await lifted.getByRole('button', { name: 'Limit lifted, retry now' }).click();
  await lifted.getByRole('button', { name: 'Codex 16% (5h) · 3% (week)' }).waitFor();
  await lifted.getByText('Codex usage limit reached').waitFor({ state: 'detached' });
  await shot(lifted, 'codex-usage-lifted');
  await lifted.context().close();

  const unknown = await open(`${lab}&codex=unknown`, '#/lab');
  const blank = unknown.getByRole('button', { name: 'Codex usage unknown' });
  await blank.waitFor();
  await hoverTip(unknown, blank, 'codex-usage-unknown');
  await unknown.context().close();

  // Two held running jobs fill half the lane, so two of four new ones wait.
  const queue = await open(`${lab}&codex=ok&lab=running`, '#/lab');
  await queue.getByLabel('Direction name').fill('Harbour');
  await queue.getByLabel('Prompt').fill('A small harbour at dawn');
  await queue.getByRole('button', { name: 'Generate 4 concepts' }).click();
  const aside = queue.getByRole('complementary', { name: 'Image queue' });
  await aside.getByRole('button', { name: 'Run next' }).waitFor();
  const waiting = aside.getByRole('listitem').filter({ hasText: 'waiting' }).last();
  await waiting.hover();
  await waiting.getByRole('button', { name: /next$/ }).waitFor();
  await shot(aside, 'codex-queue-run-next');
  await queue.context().close();
}
