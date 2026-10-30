/**
 * The Avatar view's shots for scripts/visual-check.mjs (ONLY=avatar): a
 * character never built, with its readiness, meta and rest pose measures;
 * cloth and hair freed from the arms waiting for a yes, with its shoulders
 * up close; a normalised rest pose waiting for a yes, the same on an older version
 * through the version picker, its shoulders up close and the
 * floating cards an agent's build leaves; the written VRM with its report
 * and round trip, its Preview (arms down and T-pose), the Preview with
 * nothing written, and a round trip that failed; and Settings >
 * Avatar defaults; and a rig with its own joint names, its bones picked by
 * hand in the Bone map. Mock mode's `?avatar=` sets the state (see
 * apps/web/src/ipc/mock/create-avatar-handlers.ts).
 */

const BASE = '&scenario=configured';
const TALL = { width: 1440, height: 1900 };

/** Opens the Avatar view on Elsheria and waits for her readiness. */
async function avatar(open, state = '', viewport = TALL) {
  const page = await open(`${BASE}${state ? `&avatar=${state}` : ''}`, '#/avatar', viewport);
  await page.getByRole('region', { name: 'VRM readiness' }).waitFor();
  return page;
}

export async function avatarShots({ open, shot }) {
  const fresh = await avatar(open);
  await fresh.getByText('Name at least one author.').waitFor();
  await shot(fresh, 'avatar-fresh');
  await fresh.context().close();

  // Cloth hangs on the arms: the new weights wait for a yes first.
  const weights = await avatar(open, 'reweight');
  const freeing = weights.getByRole('region', { name: 'Cloth and hair approval' });
  await freeing.waitFor();
  await weights.getByRole('img', { name: 'The after sheet' }).waitFor();
  await shot(weights, 'avatar-reweight');
  await weights.getByRole('button', { name: 'Compare the shoulders' }).click();
  await weights.getByRole('dialog', { name: 'Freed arms sheets' }).waitFor();
  await shot(weights, 'avatar-reweight-shoulders');
  await weights.context().close();

  // The weights were approved; the rest pose waits as step 2.
  const pending = await avatar(open, 'pending');
  await pending.getByRole('region', { name: 'Rest pose approval' }).waitFor();
  await pending.getByRole('img', { name: 'The after sheet' }).last().waitFor();
  await shot(pending, 'avatar-pending');
  await pending.getByRole('button', { name: 'Compare the shoulders' }).last().click();
  await pending.getByRole('dialog', { name: 'Rest pose sheets' }).waitFor();
  await shot(pending, 'avatar-shoulders');
  await pending.context().close();

  // The pose waits on v2 while v3 is newest: the picker marks it.
  const older = await avatar(open, 'older');
  await older.getByRole('combobox', { name: 'Model version' }).waitFor();
  await shot(
    older.getByRole('combobox', { name: 'Model version' }).locator('xpath=ancestor::div[2]'),
    'avatar-version-picker'
  );
  await older.getByRole('combobox', { name: 'Model version' }).selectOption('2');
  await older.getByRole('region', { name: 'Rest pose approval' }).waitFor();
  await shot(older, 'avatar-older-pending');
  await older.context().close();

  const written = await avatar(open, 'written');
  await written.getByRole('region', { name: 'Last export' }).waitFor();
  await shot(written, 'avatar-written');
  await shot(written.getByRole('region', { name: 'Export', exact: true }), 'avatar-written-export');
  // Preview loads the written file (the box humanoid fixture) with three-vrm.
  await written.getByRole('tab', { name: 'Preview' }).click();
  const preview = written.getByRole('region', { name: 'Preview' });
  await preview.getByText(/ of 55 bones$/).waitFor();
  await written.waitForTimeout(1500);
  await shot(preview, 'avatar-preview');
  await preview.getByRole('button', { name: 'T-pose' }).click();
  await written.waitForTimeout(800);
  await shot(preview, 'avatar-preview-tpose');
  await written.context().close();

  // Nothing written yet: the Preview tab points back to Export.
  const empty = await avatar(open);
  await empty.getByRole('tab', { name: 'Preview' }).click();
  await empty.getByText('No VRM written yet').waitFor();
  await shot(empty.getByRole('region', { name: 'Preview' }), 'avatar-preview-empty');
  await empty.context().close();

  // The written file did not read back: a blocker in Export, a banner in Preview.
  const failed = await avatar(open, 'round-trip');
  await failed.getByRole('region', { name: 'Last export' }).waitFor();
  await shot(failed.getByRole('region', { name: 'Export', exact: true }), 'avatar-round-trip');
  await failed.context().close();

  // A rig named its own way: the readiness asks for the bones, the Bone
  // map picks them from a search over the joints.
  const custom = await avatar(open, 'custom');
  const bones = custom.getByRole('region', { name: 'Bone map' });
  await custom.getByText('Bone names are unclear').waitFor();
  await shot(custom.getByRole('region', { name: 'VRM readiness' }), 'avatar-custom-readiness');
  await bones.scrollIntoViewIfNeeded();
  await shot(bones, 'avatar-custom-bones');
  await bones.getByRole('button', { name: 'Pick the joint for Left upper leg' }).click();
  await bones.getByRole('combobox', { name: 'Search the joints for Left upper leg' }).fill('thigh');
  await bones.getByRole('listbox', { name: 'Joints for Left upper leg' }).waitFor();
  await shot(bones, 'avatar-custom-picker');
  await bones.getByRole('option', { name: 'Left thigh' }).click();
  await bones.getByText('Left thigh').waitFor();
  await bones.getByRole('button', { name: 'Pick the joint for Right upper leg' }).click();
  await bones
    .getByRole('combobox', { name: 'Search the joints for Right upper leg' })
    .fill('right th');
  await custom.keyboard.press('Enter');
  await custom.getByText('44 bones matched by descriptive names').waitFor();
  await shot(custom, 'avatar-custom-mapped');
  await custom.context().close();

  // An agent's build waits; the card floats over every other view.
  const board = await open(`${BASE}&avatar=pending`, '#/board');
  await board.getByRole('alertdialog', { name: /new rest pose\?$/ }).waitFor();
  await shot(board, 'avatar-agent-card');
  await board.context().close();

  const freeCard = await open(`${BASE}&avatar=reweight`, '#/board');
  await freeCard.getByRole('alertdialog', { name: /cloth and hair from the arms\?$/ }).waitFor();
  await shot(freeCard, 'avatar-agent-reweight-card');
  await freeCard.context().close();

  const settings = await open(BASE, '#/settings', TALL);
  const panel = settings.getByRole('region', { name: /Avatar defaults/ });
  await panel.waitFor();
  await panel.scrollIntoViewIfNeeded();
  await shot(panel, 'avatar-settings');
  await settings.context().close();

  // The rig check opens the Avatar view for a character.
  const motion = await open(`${BASE}&motion=demo`, '#/motion');
  await motion.getByRole('button', { name: /^Clay figure/ }).click();
  const rig = motion.getByRole('region', { name: 'Rig check' });
  await rig.getByRole('button', { name: 'Open Avatar' }).waitFor();
  await shot(rig, 'avatar-rig-check');
  await motion.context().close();
}
