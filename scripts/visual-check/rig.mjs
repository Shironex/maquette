/**
 * The rig doctor's shots for scripts/visual-check.mjs (ONLY=rig): the before
 * sheet with a cell enlarged, the compare sheet with its numbers, its warning
 * and before and after enlarged, and both on the creature route. Mock mode
 * draws the pictures; the real renders are checked by the core's Blender tests.
 */

async function openCase(open, state, name) {
  const page = await open(`&scenario=configured&rig=${state}`, '#/inbox', {
    width: 1440,
    height: 1000,
  });
  await page.getByRole('button', { name }).first().click();
  await page.getByRole('heading', { name: 'Rig doctor' }).first().waitFor();
  return page;
}

export async function rigShots({ open, shot }) {
  const kept = await openCase(open, 'round2-no-change', /Duchess Vesperine/);
  await kept.getByText(/Round 2 changed nothing/).waitFor();
  await shot(kept, 'rig-round2-no-change');
  await kept.context().close();

  const before = await openCase(open, 'read', /Duchess Vesperine/);
  await before.getByRole('region', { name: 'Before sheet' }).waitFor();
  await before.getByText('Hair strands on finger bones').waitFor();
  await shot(before, 'rig-before');
  await before.getByRole('button', { name: 'Enlarge cast_orb 0.5' }).click();
  await before.getByRole('dialog', { name: 'cast_orb 0.5, enlarged' }).waitFor();
  await shot(before, 'rig-before-enlarged');
  await before.context().close();

  const compare = await openCase(open, 'compare', /Duchess Vesperine/);
  const sheet = compare.getByRole('region', { name: 'Before and draft' });
  await sheet.getByText('of 9 rows changed', { exact: false }).waitFor();
  await compare.getByText('Play those clips in Motion', { exact: false }).waitFor();
  await shot(compare, 'rig-compare');
  await sheet.getByRole('button', { name: 'Enlarge cast_orb 0.5' }).first().click();
  await compare.getByRole('dialog', { name: 'cast_orb 0.5, enlarged' }).waitFor();
  await shot(compare, 'rig-compare-enlarged');
  await compare.context().close();

  const hound = await openCase(open, 'creature-fixed', /Spiked hound/);
  await hound.getByRole('region', { name: 'Chains' }).waitFor();
  await shot(hound, 'rig-creature');
  await hound.context().close();

  const unchanged = await openCase(open, 'no-change', /Spiked hound/);
  await unchanged.getByText('The fix changed nothing', { exact: false }).waitFor();
  await shot(unchanged, 'rig-no-change');
  await unchanged.context().close();
}
