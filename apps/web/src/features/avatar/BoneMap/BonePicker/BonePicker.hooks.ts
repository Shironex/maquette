import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent } from 'react';
import { jointChoices, type JointChoice } from '@/state/avatar';
import { useAvatarStore } from '@/state/use-avatar-store';

/** The DOM id of one choice in a bone's joint list. */
export const jointDomId = (listId: string, choice: JointChoice) => `${listId}-${choice.id}`;

/**
 * A bone's joint picker: typing filters the rig's joints, the arrows move,
 * Enter maps the bone to the active choice and saves it, Escape clears the
 * search, then closes.
 */
export function useBonePicker(bone: string, onClose: () => void) {
  const listId = useId();
  const ref = useRef<HTMLDivElement>(null);
  const readiness = useAvatarStore(s => s.view?.readiness ?? null);
  const setBone = useAvatarStore(s => s.setBone);
  const saving = useAvatarStore(s => s.savingBone === bone);
  const [query, setQuery] = useState('');
  const [index, setIndex] = useState(0);
  const choices = useMemo(
    () => (readiness ? jointChoices(readiness, bone, query) : []),
    [readiness, bone, query]
  );
  const active = choices[Math.min(index, choices.length - 1)] ?? null;

  useEffect(() => ref.current?.querySelector('input')?.focus(), []);

  const move = (to: number) => {
    const next = Math.max(0, Math.min(choices.length - 1, to));
    setIndex(next);
    const choice = choices[next];
    if (choice)
      document.getElementById(jointDomId(listId, choice))?.scrollIntoView({ block: 'nearest' });
  };

  const pick = (choice: JointChoice) => {
    if (saving) return;
    void setBone(bone, choice.value).then(onClose);
  };

  const onKey = (e: KeyboardEvent) => {
    const at = active ? choices.indexOf(active) : 0;
    if (e.key === 'ArrowDown') move(at + 1);
    else if (e.key === 'ArrowUp') move(at - 1);
    else if (e.key === 'Enter' && active) pick(active);
    else if (e.key === 'Escape') {
      if (query) {
        setQuery('');
        setIndex(0);
      } else onClose();
    } else return;
    e.preventDefault();
    e.stopPropagation();
  };

  return {
    listId,
    ref,
    query,
    search: (value: string) => {
      setQuery(value);
      setIndex(0);
    },
    choices,
    active,
    saving,
    pick,
    onKey,
  };
}
