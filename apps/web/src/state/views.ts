import type { IconName, NavRailPlanned } from '@maquette/ui';
import type { ProjectTarget } from '@/ipc/client';

export type ViewId =
  | 'board'
  | 'inbox'
  | 'lab'
  | 'pipeline'
  | 'inspector'
  | 'motion'
  | 'avatar'
  | 'timeline'
  | 'page'
  | 'library'
  | 'animations'
  | 'settings';

export interface ViewDef {
  id: ViewId;
  label: string;
  icon: IconName;
}

export const TOOL_VIEWS: readonly ViewDef[] = [
  { id: 'board', label: 'Board', icon: 'squares-four' },
  { id: 'inbox', label: 'Inbox', icon: 'tray' },
  { id: 'lab', label: 'Lab', icon: 'flask' },
  { id: 'pipeline', label: 'Pipeline', icon: 'flow-arrow' },
  { id: 'inspector', label: 'Inspector', icon: 'cube-focus' },
  { id: 'motion', label: 'Motion', icon: 'person-simple-walk' },
  { id: 'avatar', label: 'Avatar', icon: 'person-arms-spread' },
  { id: 'timeline', label: 'Timeline', icon: 'film-strip' },
  { id: 'page', label: 'Page', icon: 'browser' },
];

/** The workspace's libraries, shared by every project: models and animation clips. */
export const LIBRARY_VIEWS: readonly ViewDef[] = [
  { id: 'library', label: 'Models', icon: 'cube' },
  { id: 'animations', label: 'Animations', icon: 'person-simple' },
];

/** Views a game project has no use for: it ships no web page. */
const WEB_ONLY: readonly ViewId[] = ['timeline', 'page'];

/** The rail's views for a project that ships to `target`. */
export function toolViewsFor(target: ProjectTarget | undefined): readonly ViewDef[] {
  return target === 'game' ? TOOL_VIEWS.filter(v => !WEB_ONLY.includes(v.id)) : TOOL_VIEWS;
}

/** Whether a project that ships to `target` shows `view`. */
export function showsView(target: ProjectTarget | undefined, view: ViewId): boolean {
  return target !== 'game' || !WEB_ONLY.includes(view);
}

/** What the rail lists as planned for a game project. */
export function plannedViewsFor(target: ProjectTarget | undefined): readonly NavRailPlanned[] {
  return target === 'game'
    ? [{ id: 'game-preview', label: 'Game preview', icon: 'game-controller', tag: 'later' }]
    : [];
}

export const SETTINGS_VIEW: ViewDef = { id: 'settings', label: 'Settings', icon: 'gear-six' };

const ALL = [...TOOL_VIEWS, ...LIBRARY_VIEWS, SETTINGS_VIEW].map(v => v.id);

/** `#/settings` to `settings`; anything else falls back to the Board. */
export function viewFromHash(hash: string): ViewId {
  const id = hash.replace(/^#\/?/, '').split(/[/?]/)[0] ?? '';
  return (ALL as string[]).includes(id) ? (id as ViewId) : 'board';
}

export function hashForView(view: ViewId): string {
  return `#/${view}`;
}
