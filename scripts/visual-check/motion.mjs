/**
 * The Motion view's shots for scripts/visual-check.mjs (ONLY=motion): a
 * rigged figure playing a clip, a prop's states, a rig without clips, a
 * project where nothing moves, the scene over its budget with the clips in
 * it, and the bone map. Mock mode's `?motion=` demo models, generated
 * headless in Blender, load in the viewport.
 */

const DEMO = '&scenario=configured&motion=';

/** Opens the Motion view on a demo and waits for the list. */
async function motion(open, demo) {
  const page = await open(`${DEMO}${demo}`, '#/motion');
  await page.getByRole('region', { name: 'Models that move' }).waitFor();
  return page;
}

/** Waits for the viewport's model, then holds the clip at `frame`. */
async function holdAt(page, frame) {
  await page.getByRole('slider', { name: 'Playhead' }).waitFor();
  await page.waitForTimeout(1200);
  const playing = page.getByRole('button', { name: 'Pause' });
  if (await playing.isVisible()) await playing.click();
  await page.getByRole('button', { name: 'Go to In' }).click();
  for (let i = 0; i < frame; i += 1) await page.keyboard.press('.');
  await page.evaluate(() => document.activeElement?.blur());
  await page.waitForTimeout(400);
}

export async function motionShots({ open, shot }) {
  const main = await motion(open, 'demo');
  await main.getByRole('button', { name: /^Walk loop/ }).click();
  await holdAt(main, 18);
  await main.getByText(/^Walk loop · frame 18$/).waitFor();
  await shot(main, 'motion-main');

  await main.getByRole('button', { name: 'Map bones' }).click();
  await main.getByRole('dialog', { name: /^Retarget a library clip onto / }).waitFor();
  await shot(main, 'motion-retarget');
  await main.getByRole('button', { name: 'Cancel' }).click();

  await main.getByRole('button', { name: /^Switch cabinet/ }).click();
  await main.getByText(/^Door open · end of the /).waitFor();
  await main.waitForTimeout(1200);
  await shot(main, 'motion-states');

  const rest = await motion(open, 'rest');
  await rest.getByText('No clips on Clay figure yet').waitFor();
  await rest.waitForTimeout(1200);
  await shot(rest, 'motion-no-clips');

  const still = await motion(open, 'still');
  await still.getByText(/ could move$/).waitFor();
  await shot(still, 'motion-empty');

  const over = await motion(open, 'over');
  await over.getByRole('button', { name: /^Character\s*75 bones/ }).click();
  await over
    .getByRole('alert')
    .filter({ hasText: /over its .+ budget/ })
    .waitFor();
  await holdAt(over, 40);
  await shot(over, 'motion-over-budget');
}
