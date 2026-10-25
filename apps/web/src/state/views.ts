import type { IconName } from '@maquette/ui';

export type ViewId = 'board' | 'inbox' | 'lab' | 'pipeline' | 'inspector' | 'settings';

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
];

export const SETTINGS_VIEW: ViewDef = { id: 'settings', label: 'Settings', icon: 'gear-six' };

const ALL = [...TOOL_VIEWS, SETTINGS_VIEW].map(v => v.id);

/** `#/settings` to `settings`; anything else falls back to the Board. */
export function viewFromHash(hash: string): ViewId {
  const id = hash.replace(/^#\/?/, '').split(/[/?]/)[0] ?? '';
  return (ALL as string[]).includes(id) ? (id as ViewId) : 'board';
}

export function hashForView(view: ViewId): string {
  return `#/${view}`;
}
