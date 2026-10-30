import { useEffect, useRef, useState, type KeyboardEvent } from 'react';
import type { LibraryItem } from '@/ipc/client';
import { tagEntries } from '@/state/library-actions';
import { cleanTag, selectionTags, tagMatches } from '@/state/library-selection';

/** The Tag popover's field, what the picked models carry, and adding or taking off a tag. */
export function useTagPopover(
  picked: readonly LibraryItem[],
  known: readonly string[],
  onClose: () => void
) {
  const ref = useRef<HTMLDivElement>(null);
  const [typed, setTyped] = useState('');
  const [busy, setBusy] = useState(false);
  const ids = picked.map(i => i.entry.id);
  const text = cleanTag(typed);

  useEffect(() => {
    const away = (e: PointerEvent) => {
      const target = e.target as Node;
      if (
        ref.current &&
        !ref.current.contains(target) &&
        !(target as Element).closest?.('[data-tag-toggle]')
      )
        onClose();
    };
    document.addEventListener('pointerdown', away);
    return () => document.removeEventListener('pointerdown', away);
  }, [onClose]);

  const change = async (add: string[], remove: string[]) => {
    setBusy(true);
    const ok = await tagEntries(ids, add, remove);
    setBusy(false);
    if (ok && add.includes(text)) setTyped('');
  };

  return {
    ref,
    typed,
    setTyped,
    busy,
    text,
    tags: selectionTags(picked),
    matches: tagMatches(typed, known, picked),
    isNew: Boolean(text) && !known.includes(text),
    add: (tag: string) => void change([tag], []),
    remove: (tag: string) => void change([], [tag]),
    onKeyDown: (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        onClose();
        return;
      }
      if (e.key !== 'Enter' || !text) return;
      e.preventDefault();
      void change([text], []);
    },
  };
}
