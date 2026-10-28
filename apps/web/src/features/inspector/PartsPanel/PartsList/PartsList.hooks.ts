import { useState } from 'react';
import type { PartRow } from '@/ipc/client';

/** A part's rename in place: double click starts it, Enter or a blur keeps it. */
export function usePartRename(row: PartRow, onRename: (id: string, name: string) => void) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(row.name);
  const finish = (keep: boolean) => {
    setEditing(false);
    if (keep && draft.trim() && draft !== row.name) onRename(row.id, draft);
    else setDraft(row.name);
  };
  return {
    editing,
    draft,
    setDraft,
    start: () => {
      setDraft(row.name);
      setEditing(true);
    },
    finish,
  };
}
