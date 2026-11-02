import { useState } from 'react';
import type { ShipTarget, SiteTarget } from '@/ipc/client';

/** The folder fields as typed: empty posters or data means none. */
export interface FolderDraft {
  models: string;
  posters: string;
  data: string;
}

export const draftOf = (t: ShipTarget): FolderDraft => ({
  models: t.models,
  posters: t.posters ?? '',
  data: t.data ?? '',
});

/** The draft as `maquette.site.json` takes it: trimmed, empty folders to none. */
export function targetOf(draft: FolderDraft): SiteTarget {
  const or = (text: string) => text.trim().replace(/\/+$/, '') || null;
  return { models: or(draft.models) ?? '', posters: or(draft.posters), data: or(draft.data) };
}

/** The one line every field shows while there is no repo; the button beside them says how. */
export const NEEDS_REPO = 'Waits for a site repo';

/** Why the folders cannot be saved, or null when they can. */
export function folderBlock(draft: FolderDraft, target: ShipTarget): string | null {
  if (!target.repo) return NEEDS_REPO;
  const next = targetOf(draft);
  if (!next.models) return 'Name the models folder';
  const same =
    next.models === target.models && next.posters === target.posters && next.data === target.data;
  if (same && target.configured) return 'Nothing changed yet';
  return null;
}

/** The folder fields, reset whenever the saved target changes. */
export function useFolderDraft(target: ShipTarget) {
  const key = `${target.repo ?? ''} ${target.models} ${target.posters ?? ''} ${target.data ?? ''}`;
  const [held, setHeld] = useState(() => ({ key, draft: draftOf(target) }));
  // A saved target (or another project) replaces what was typed.
  if (held.key !== key) setHeld({ key, draft: draftOf(target) });
  const draft = held.key === key ? held.draft : draftOf(target);
  const change = (patch: Partial<FolderDraft>) =>
    setHeld(h => ({ ...h, draft: { ...h.draft, ...patch } }));
  return { draft, change, block: folderBlock(draft, target) };
}
