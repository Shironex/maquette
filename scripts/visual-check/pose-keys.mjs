/**
 * The pose keys shots for scripts/visual-check.mjs (ONLY=posekeys): the
 * demo recipe that lays pose keys over a slash and a block, open in the
 * editor with its pose key change picked and the playhead on a chest key:
 * the whole editor, the lanes with a diamond per key and per bone, the turn
 * gizmo and bone dots on the rough preview with its bar, the bone picker,
 * the form with its ease picker and joint limits line, and a turn past a
 * limit laid on the preview.
 */

/** Opens the pose keys demo in the editor, its change picked, held on frame `frame`. */
async function openPoseKeys(open, frame) {
  const page = await open('&scenario=configured&motion=demo&library=demo', '#/motion');
  await page.getByRole('region', { name: 'Models that move' }).waitFor();
  await page.getByRole('tab', { name: 'Recipes' }).click();
  await page
    .getByRole('list', { name: 'Project recipes' })
    .getByRole('listitem')
    .filter({ hasText: 'Slash with a lean' })
    .getByRole('button', { name: 'Edit' })
    .click();
  await page.getByRole('img', { name: /rough preview$/ }).waitFor();
  await page.waitForTimeout(2500);
  await page.getByRole('button', { name: 'Pause' }).click();
  await page.getByRole('button', { name: 'Pose keys l-1' }).click();
  await page.getByRole('slider', { name: 'Playhead' }).fill(String(frame));
  await page.getByRole('group', { name: 'Change l-1 settings' }).waitFor();
  await page.evaluate(() => document.activeElement?.blur());
  await page.waitForTimeout(900);
  return page;
}

export async function poseKeyShots({ open, shot }) {
  const page = await openPoseKeys(open, 20);
  await shot(page, 'motion-recipe-pose-keys');
  await shot(page.getByRole('region', { name: 'Timeline' }), 'motion-recipe-pose-lane');
  await shot(page.getByRole('figure', { name: 'Recipe preview' }), 'motion-recipe-pose-gizmo');
  await shot(page.getByRole('toolbar', { name: 'Turn gizmo' }), 'motion-recipe-pose-bar');
  await shot(page.getByRole('group', { name: 'Bones' }), 'motion-recipe-pose-bones');
  const form = page.getByRole('group', { name: 'Change l-1 settings' });
  await shot(form, 'motion-recipe-pose-form');

  // A turn past the chest's twist limit warns, and Step picked in the ease picker.
  const twist = form.getByLabel('Turn Y');
  await twist.fill('60');
  await twist.press('Enter');
  await form.getByRole('button', { name: 'Step' }).click();
  await page.evaluate(() => document.activeElement?.blur());
  await page.waitForTimeout(600);
  await shot(form, 'motion-recipe-pose-ease');
  // The preview lays the 60 degree twist on the chest at once.
  await shot(page.getByRole('figure', { name: 'Recipe preview' }), 'motion-recipe-pose-turned');
  await page.context().close();
}
