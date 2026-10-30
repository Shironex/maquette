/**
 * The Avatar view's shots for scripts/visual-check.mjs (ONLY=avatar): a
 * character never built, with its readiness, meta and rest pose measures;
 * a normalised rest pose waiting for a yes, its shoulders up close and the
 * floating card an agent's build leaves; the written VRM; and Settings >
 * Avatar defaults. Mock mode's `?avatar=` sets the state (see
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

  const pending = await avatar(open, 'pending');
  await pending.getByRole('region', { name: 'Rest pose approval' }).waitFor();
  await pending.getByRole('img', { name: 'The after sheet' }).waitFor();
  await shot(pending, 'avatar-pending');
  await pending.getByRole('button', { name: 'Compare the shoulders' }).click();
  await pending.getByRole('dialog', { name: 'Rest pose sheets' }).waitFor();
  await shot(pending, 'avatar-shoulders');
  await pending.context().close();

  const written = await avatar(open, 'written');
  await written.getByRole('region', { name: 'Last export' }).waitFor();
  await shot(written, 'avatar-written');
  await shot(written.getByRole('region', { name: 'Export', exact: true }), 'avatar-written-export');
  await written.context().close();

  // An agent's build waits; the card floats over every other view.
  const board = await open(`${BASE}&avatar=pending`, '#/board');
  await board.getByRole('alertdialog', { name: /new rest pose\?$/ }).waitFor();
  await shot(board, 'avatar-agent-card');
  await board.context().close();

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
