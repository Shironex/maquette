import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent } from 'react';
import {
  filterGroups,
  optionDomId,
  pickable,
  type CompareGroup,
  type CompareOption,
} from '../compare-options';

/**
 * The search box and the keyboard of the compare list: typing filters, the
 * arrows move through the matches, Enter picks the active one, Escape
 * clears the search and then closes. A click outside closes too.
 */
export function useComparePopover(groups: CompareGroup[], onClose: () => void) {
  const listId = useId();
  const rootRef = useRef<HTMLDivElement>(null);
  const [query, setQuery] = useState('');
  const [index, setIndex] = useState(0);
  const shown = useMemo(() => filterGroups(groups, query), [groups, query]);
  const options = useMemo(() => pickable(shown), [shown]);
  const active = options[Math.min(index, options.length - 1)] ?? null;

  useEffect(() => {
    // On open the list shows the current comparison, so scroll it into view.
    rootRef.current?.querySelector('[aria-current="true"]')?.scrollIntoView({ block: 'nearest' });
    rootRef.current?.querySelector('input')?.focus();
    const onDown = (e: PointerEvent) => {
      // The button that opened the list sits in the same parent, and its click toggles.
      const within = rootRef.current?.parentElement ?? rootRef.current;
      if (within && !within.contains(e.target as Node)) onClose();
    };
    window.addEventListener('pointerdown', onDown);
    return () => window.removeEventListener('pointerdown', onDown);
  }, [onClose]);

  const pick = (option: CompareOption) => {
    option.onSelect();
    onClose();
  };

  const move = (to: number) => {
    const next = Math.max(0, Math.min(options.length - 1, to));
    setIndex(next);
    const option = options[next];
    if (option) {
      document.getElementById(optionDomId(listId, option))?.scrollIntoView({ block: 'nearest' });
    }
  };

  const search = (value: string) => {
    setQuery(value);
    setIndex(0);
  };

  const onKeyDown = (e: KeyboardEvent) => {
    const at = active ? options.indexOf(active) : 0;
    if (e.key === 'ArrowDown') move(at + 1);
    else if (e.key === 'ArrowUp') move(at - 1);
    else if (e.key === 'Home') move(0);
    else if (e.key === 'End') move(options.length - 1);
    else if (e.key === 'Enter' && active) pick(active);
    else if (e.key === 'Escape') {
      if (query) search('');
      else onClose();
    } else return;
    e.preventDefault();
  };

  return { rootRef, listId, query, search, shown, active, onKeyDown, pick };
}
