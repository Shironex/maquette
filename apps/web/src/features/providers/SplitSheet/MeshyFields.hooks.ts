import { useState, type KeyboardEvent } from 'react';
import type { MeshySplit } from '@/ipc/client';
import { meshyNames } from '@/state/split';

/** The part name being typed: Enter or a comma adds it, a blur too. */
export function usePartNames(
  names: readonly string[],
  onChange: (patch: Partial<MeshySplit>) => void
) {
  const [typed, setTyped] = useState('');
  const add = () => {
    const next = meshyNames([...names, ...typed.split(',')]);
    if (next.length > names.length) onChange({ parts: next });
    setTyped('');
  };
  const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key !== 'Enter' && e.key !== ',') return;
    e.preventDefault();
    add();
  };
  const onBlur = () => {
    if (typed.trim()) add();
  };
  const remove = (name: string) => onChange({ parts: names.filter(n => n !== name) });
  return { typed, setTyped, onKeyDown, onBlur, remove };
}
