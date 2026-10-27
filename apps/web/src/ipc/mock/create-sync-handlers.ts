/**
 * A workspace shared with another computer, in mock mode (`?sync=`):
 * - `held`: the other computer has the workspace open right now, so the
 *   app asks before it opens it (see create-workspace-handlers.ts).
 * - `repo`: Moon Island's site repo was picked on the other computer, so
 *   this one asks where it is.
 */
import type { ProjectManifest } from '../client';

export const OTHER_COMPUTER = 'Studio PC';
export const IN_USE_MESSAGE = `This workspace is open on ${OTHER_COMPUTER}. Close Maquette there first, or Syncthing may create conflict copies.`;

/** Moon Island as another computer left it: its repo is not here. */
export function syncProjects(projects: ProjectManifest[], sync: string | null): ProjectManifest[] {
  if (sync !== 'repo') return projects;
  return projects.map(p =>
    p.id === 'moon-island'
      ? { ...p, siteRepo: null, siteRepoName: 'moon-island-site', siteRepoState: 'elsewhere' }
      : p
  );
}
