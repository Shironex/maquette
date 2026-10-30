import { useState, type KeyboardEvent } from 'react';
import { changeCollection } from '@/state/library-actions';

/** The New collection field: closed, or open with the name typed so far. */
export function useNewCollection(entries: readonly string[]) {
  const [naming, setNaming] = useState(false);
  const [name, setName] = useState('');
  const done = () => {
    setNaming(false);
    setName('');
  };
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.stopPropagation();
      done();
      return;
    }
    if (e.key !== 'Enter' || !name.trim()) return;
    void changeCollection({ op: 'create', name: name.trim(), entries: [...entries] }).then(
      ok => ok && done()
    );
  };
  return {
    naming,
    name,
    setName,
    open: () => setNaming(true),
    onBlur: () => !name.trim() && done(),
    onKeyDown,
  };
}
