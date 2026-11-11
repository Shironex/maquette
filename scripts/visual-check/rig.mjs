/**
 * The rig doctor's shots for scripts/visual-check.mjs (ONLY=rig): the before
 * sheet with a cell enlarged, the compare sheet with its numbers, its warning
 * and before and after enlarged, and both on the creature route, and an
 * agent's fixed draft with its card answered in place and another case's card
 * floating clear of Add, also at the minimum window size and over an Inbox
 * model's Accept, and a walk that goes against its face in the Inbox checks. Mock mode draws the pictures; the real renders are checked
 * by the core's Blender tests.
 */

import { assertClear } from './floating.mjs';

async function openCase(open, state, name, viewport = { width: 1440, height: 1000 }) {
  const page = await open(`&scenario=configured&rig=${state}`, '#/inbox', viewport);
  await page.getByRole('button', { name }).first().click();
  await page.getByRole('heading', { name: 'Rig doctor' }).first().waitFor();
  return page;
}

/** An agent's two drafts: the open case's card stays home, the other floats above Add. */
async function agentDraftShots(open, shot) {
  for (const [viewport, name] of [
    [{ width: 1440, height: 1000 }, 'rig-agent-card'],
    [{ width: 1120, height: 720 }, 'rig-agent-card-min'],
  ]) {
    // The Inbox header's case button; a floating card's buttons name the hound too.
    const page = await openCase(open, 'agent-fixed', /^Spiked hound: draft v2 waits/, viewport);
    const add = page.getByRole('button', { name: 'Add as v2' });
    await add.waitFor();
    await page.getByRole('alertdialog', { name: 'Add Duchess Vesperine v2?' }).waitFor();
    if (await page.getByRole('alertdialog', { name: 'Add Spiked hound v2?' }).count()) {
      throw new Error('The open case still floats its own card');
    }
    await assertClear(page, add, 'Add as v2');
    await shot(page, name);
    await page.context().close();
  }

  const inbox = await open('&scenario=configured&rig=agent-fixed', '#/inbox', {
    width: 1440,
    height: 900,
  });
  const accept = inbox.getByRole('button', { name: /^Accept/ }).last();
  await accept.waitFor();
  await inbox.getByRole('alertdialog').first().waitFor();
  await assertClear(inbox, accept, 'Accept in the Inbox');
  await shot(inbox, 'rig-agent-card-inbox');
  await inbox.context().close();
}

export async function rigShots({ open, shot }) {
  await agentDraftShots(open, shot);

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

  // A walk clip that carries the model backwards is flagged in the Inbox checks.
  const walk = await open('&scenario=configured&inbox=walk', '#/inbox');
  await walk.getByText('Ash walker').first().click();
  await walk.getByText('Walk travels against the face').waitFor();
  await shot(walk, 'inbox-walk-against-face');
  await walk.context().close();
}
