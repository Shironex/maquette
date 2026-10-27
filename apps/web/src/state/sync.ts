/**
 * Sharing a workspace between computers with a sync tool: the words for its
 * conflict copies, the `.stignore` Maquette recommends, and when to ask
 * where a project's repo is on this computer.
 */
import type { ProjectManifest, SyncConflict } from '@/ipc/client';
import { fileName } from '@/state/pipeline';

/** "Syncthing kept a conflict copy of project.json" */
export function conflictTitle(conflicts: readonly SyncConflict[]): string {
  if (conflicts.length === 1) {
    return `Syncthing kept a conflict copy of ${fileName(conflicts[0]?.original ?? '')}`;
  }
  return `Syncthing kept ${conflicts.length} conflict copies of Maquette files`;
}

/** Changes when the set of copies does, so a dismissed warning comes back for a new one. */
export function conflictKey(conflicts: readonly SyncConflict[]): string {
  return conflicts.map(c => c.path).join('\n');
}

/**
 * What to put in the shared folder's `.stignore`. The cache is rebuilt on
 * each computer and OS clutter means nothing on the other one. The lock is
 * left in on purpose: it is how the other computer knows this one has the
 * workspace open.
 */
export function stignore(cache: string): string {
  return [
    '// Rebuilt on each computer',
    `/${cache}`,
    '// Files caught mid save',
    '(?d).*.tmp',
    '// Folder clutter from macOS and Windows',
    '(?d).DS_Store',
    '(?d)._*',
    '(?d)Thumbs.db',
    '(?d)desktop.ini',
    '// .maquette-lock.json stays synced on purpose',
    '',
  ].join('\n');
}

/** True while this computer should be asked where the project's repo is. */
export function needsRepo(project: ProjectManifest | null | undefined): boolean {
  const state = project?.siteRepoState;
  return state === 'elsewhere' || state === 'gone';
}

/** The repo prompt's text for `project`. */
export function repoQuestion(project: ProjectManifest): string {
  const kind = project.target === 'game' ? 'game repo' : 'site repo';
  const users =
    project.target === 'game'
      ? 'Import models, the Game export and Ship'
      : 'Ship, the Page view and Preview on site';
  const named = project.siteRepoName ? ` (${project.siteRepoName})` : '';
  if (project.siteRepoState === 'gone') {
    return `${project.name}'s ${kind}${named} is not at ${project.siteRepo ?? 'its old place'} any more. Choose where it is now so ${users} can find it.`;
  }
  return `${project.name}'s ${kind}${named} was picked on another computer. Choose where it is on this one so ${users} can find it. Each computer keeps its own path.`;
}
