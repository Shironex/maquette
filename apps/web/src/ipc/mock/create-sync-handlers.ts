/**
 * A workspace shared with another computer, in mock mode (`?sync=`):
 * - `held`: the other computer has the workspace open right now, so the
 *   app asks before it opens it (see create-workspace-handlers.ts).
 * - `repo`: Frostveil's site repo was picked on the other computer, so
 *   this one asks where it is.
 * - `conflict`: the sync tool kept conflict copies of two data files.
 */
import type { ProjectManifest, SyncConflict } from '../client';

export const OTHER_COMPUTER = 'Studio PC';
export const IN_USE_MESSAGE = `This workspace is open on ${OTHER_COMPUTER}. Close Maquette there first, or Syncthing may create conflict copies.`;

const STAMP = 'sync-conflict-20261027-101500-PCSTUDIO';

/** The conflict copies `?sync=conflict` reports. */
export function mockConflicts(): SyncConflict[] {
  return [
    {
      path: `projects/frostveil/project.${STAMP}.json`,
      original: 'projects/frostveil/project.json',
    },
    { path: `maquette.${STAMP}.json`, original: 'maquette.json' },
  ];
}

/** Frostveil as another computer left it: its repo is not here. */
export function syncProjects(projects: ProjectManifest[], sync: string | null): ProjectManifest[] {
  if (sync !== 'repo') return projects;
  return projects.map(p =>
    p.id === 'frostveil'
      ? { ...p, siteRepo: null, siteRepoName: 'frostveil-site', siteRepoState: 'elsewhere' }
      : p
  );
}

export function createSyncHandlers(sync: string | null) {
  const conflicts = sync === 'conflict' ? mockConflicts() : [];
  return {
    workspace_sync_conflicts: () => conflicts,
    // The real command selects the file in Finder or Explorer.
    workspace_reveal: () => null,
  };
}
