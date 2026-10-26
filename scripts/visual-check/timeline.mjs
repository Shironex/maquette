/**
 * The Timeline view's shots for scripts/visual-check.mjs (ONLY=timeline):
 * the switch cabinet's lanes with a scroll range selected and the page
 * preview scrolled to the playhead, an explode key selected, Add track
 * with paths still to come, and a project without camera stops. Mock
 * mode's `?timeline=demo` cabinet and its dev page serve them.
 */

const RUNNING = '&scenario=configured&page=running&timeline=';

/** Opens the Timeline view and waits for the page preview to answer the bridge. */
async function timeline(open, demo) {
  const page = await open(`${RUNNING}${demo}`, '#/timeline');
  await page.getByRole('heading', { name: 'Scroll timeline' }).waitFor();
  await page.getByTitle(/page from the site dev page$/).waitFor({ timeout: 20_000 });
  return page;
}

/** Moves the playhead to `percent` from the keyboard, as a person scrubbing would. */
async function scrubTo(page, percent) {
  const slider = page.getByRole('slider', { name: 'Playhead' });
  await slider.focus();
  await page.keyboard.press('Home');
  for (let i = 0; i < Math.floor(percent / 10); i += 1) await page.keyboard.press('PageDown');
  for (let i = 0; i < percent % 10; i += 1) await page.keyboard.press('ArrowRight');
  await page.evaluate(() => document.activeElement?.blur());
  await page.waitForTimeout(1500);
}

export async function timelineShots({ open, shot }) {
  const main = await timeline(open, 'demo');
  await main.getByRole('button', { name: /^Switch cabinet: Breakers on from / }).click();
  await scrubTo(main, 62);
  await main.getByRole('heading', { name: 'Switch cabinet · Breakers on' }).waitFor();
  await shot(main, 'timeline-main');

  await main.getByRole('button', { name: 'Switch cabinet apart at Pullback' }).click();
  await main.getByRole('heading', { name: 'Switch cabinet · explode at Pullback' }).waitFor();
  await shot(main, 'timeline-explode');

  await main.getByRole('button', { name: 'Add track' }).click();
  const path = main.getByRole('menuitem', { name: 'Path' });
  await path.hover();
  await main.getByRole('tooltip').filter({ hasText: 'Paths come in the next view' }).waitFor();
  await shot(main, 'timeline-paths');
  await main.context().close();

  const empty = await open('&scenario=configured&timeline=empty', '#/timeline');
  await empty.getByRole('heading', { name: /has no camera stops yet$/ }).waitFor();
  await shot(empty, 'timeline-empty');
  await empty.context().close();
}
