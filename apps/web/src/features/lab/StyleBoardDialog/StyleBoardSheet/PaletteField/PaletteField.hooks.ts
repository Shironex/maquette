import { useState, type KeyboardEvent } from 'react';
import { addColours } from '@/state/lab-style';

/** The typed colour, whether it reads as one, and Add on Enter. */
export function usePaletteField(palette: readonly string[], onChange: (next: string[]) => void) {
  const [text, setText] = useState('');
  const [bad, setBad] = useState(false);
  const commit = (value: string) => {
    if (!value.trim()) return;
    const next = addColours(palette, value);
    if (!next) {
      setBad(true);
      return;
    }
    onChange(next);
    setText('');
    setBad(false);
  };
  const onKey = (e: KeyboardEvent<HTMLInputElement>) => {
    // The submit chord saves the board; a plain Enter adds the colour.
    if (e.key !== 'Enter' || e.metaKey || e.ctrlKey) return;
    e.preventDefault();
    commit(text);
  };
  const type = (value: string) => {
    setText(value);
    setBad(false);
  };
  const remove = (colour: string) => onChange(palette.filter(c => c !== colour));
  return { text, type, bad, commit, onKey, remove };
}
