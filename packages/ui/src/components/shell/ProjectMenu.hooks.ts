import {
  useEffect,
  useEffectEvent,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type RefObject,
} from 'react';
import type { ProjectMenuProject } from './ProjectMenu';

function matches(project: ProjectMenuProject, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return `${project.name} ${project.meta ?? ''}`.toLowerCase().includes(q);
}

interface ProjectMenuOptions {
  projects: readonly ProjectMenuProject[];
  searchFrom: number;
  anchorRef?: RefObject<HTMLElement | null>;
  onSelect: (id: string) => void;
  onClose: () => void;
}

/**
 * The menu's search, its filtered list and its keyboard: arrow keys move,
 * Home and End jump, Enter picks, Escape closes and typing searches. Focus
 * lands once on open and a pointer down outside closes it.
 */
export function useProjectMenu({
  projects,
  searchFrom,
  anchorRef,
  onSelect,
  onClose,
}: ProjectMenuOptions) {
  const ref = useRef<HTMLDivElement>(null);
  const search = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState('');
  const searching = projects.length > searchFrom;
  const shown = useMemo(
    () => (searching ? projects.filter(p => matches(p, query)) : projects),
    [projects, query, searching]
  );

  const items = () => [...(ref.current?.querySelectorAll<HTMLElement>('[data-menu-item]') ?? [])];

  // Effect events read the latest props, so the open effect runs once: the
  // list changing must not steal focus back.
  const focusOnOpen = useEffectEvent(() => {
    if (searching) search.current?.focus();
    else (ref.current?.querySelector<HTMLElement>('[aria-checked="true"]') ?? items()[0])?.focus();
  });
  const onPointerDown = useEffectEvent((e: PointerEvent) => {
    const target = e.target as Node;
    if (ref.current?.contains(target) || anchorRef?.current?.contains(target)) return;
    onClose();
  });

  useEffect(() => {
    focusOnOpen();
    const onDown = (e: PointerEvent) => onPointerDown(e);
    window.addEventListener('pointerdown', onDown);
    return () => window.removeEventListener('pointerdown', onDown);
  }, []);

  const move = (to: 'next' | 'prev' | 'first' | 'last') => {
    const list = items();
    if (list.length === 0) return;
    const at = list.indexOf(document.activeElement as HTMLElement);
    const index =
      to === 'first'
        ? 0
        : to === 'last'
          ? list.length - 1
          : at < 0
            ? to === 'next'
              ? 0
              : list.length - 1
            : to === 'next'
              ? at + 1
              : at - 1;
    if (index < 0 && searching) {
      search.current?.focus();
      return;
    }
    list[(index + list.length) % list.length]?.focus();
  };

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const inSearch = e.target === search.current;
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onClose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      move('next');
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      move('prev');
    } else if ((e.key === 'Home' || e.key === 'End') && !inSearch) {
      e.preventDefault();
      move(e.key === 'Home' ? 'first' : 'last');
    } else if (e.key === 'Enter' && inSearch) {
      e.preventDefault();
      const first = shown[0];
      if (first) onSelect(first.id);
    } else if (
      searching &&
      !inSearch &&
      e.key.length === 1 &&
      !e.metaKey &&
      !e.ctrlKey &&
      !e.altKey
    ) {
      // Typing on a row searches.
      e.preventDefault();
      setQuery(q => q + e.key);
      search.current?.focus();
    }
  };

  return { ref, search, query, setQuery, shown, searching, onKeyDown };
}
