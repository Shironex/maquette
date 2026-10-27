/**
 * Sharing a workspace with another computer, for scripts/visual-check.mjs
 * (ONLY=sync): the question asked while another computer has the workspace
 * open, the question for a repo picked on the other computer, the warning
 * for conflict copies the sync tool kept, and the Settings help with the
 * recommended `.stignore`. Mock mode's `?sync=` states serve them.
 */

export async function syncShots({ open, shot }) {
  const held = await open('&scenario=configured&sync=held', '#/board');
  await held.getByRole('dialog', { name: 'Open this workspace here too?' }).waitFor();
  await shot(held, 'sync-workspace-in-use');
  await held.context().close();

  const repo = await open('&scenario=configured&sync=repo', '#/board');
  await repo
    .getByRole('dialog', { name: "Where is this project's repo on this computer?" })
    .waitFor();
  await shot(repo, 'sync-repo-prompt');
  await repo.context().close();

  const conflict = await open('&scenario=configured&sync=conflict', '#/board');
  await conflict.getByText('Syncthing kept 2 conflict copies of Maquette files').waitFor();
  await shot(conflict, 'sync-conflict-warning');
  await conflict.context().close();

  const settings = await open('&scenario=configured', '#/settings');
  const help = settings.getByText('Sync with another computer');
  await help.scrollIntoViewIfNeeded();
  await help.click();
  await settings.getByRole('button', { name: 'Copy .stignore' }).scrollIntoViewIfNeeded();
  await shot(settings, 'sync-settings-help');
  await settings.context().close();
}
