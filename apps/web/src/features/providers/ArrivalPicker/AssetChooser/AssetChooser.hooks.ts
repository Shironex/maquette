import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent } from 'react';
import { assetChoices, askingArrivals, type AssetChoice } from '@/state/downloads';
import { stageLabels } from '@/state/game';
import { useAppStore } from '@/state/use-app-store';
import { useDownloadsStore } from '@/state/use-downloads-store';

/** The DOM id of one asset in the chooser's list. */
export const choiceDomId = (listId: string, asset: string) => `${listId}-${asset}`;

/**
 * The asset chooser's search and keyboard: typing filters the project's
 * assets (the ones a send waits on first), the arrows move, Enter attaches
 * the active one and Escape clears the search, then closes. After an
 * attach it moves on to the next download asking, with a fresh search.
 */
export function useAssetChooser(arrivalId: string) {
  const listId = useId();
  const ref = useRef<HTMLDivElement>(null);
  const arrival = useDownloadsStore(s => s.status?.arrivals.find(a => a.id === arrivalId));
  const pending = useDownloadsStore(s => s.status?.pending);
  const status = useDownloadsStore(s => s.status);
  const busy = useDownloadsStore(s => s.busy.includes(arrivalId));
  const attach = useDownloadsStore(s => s.attach);
  const close = useDownloadsStore(s => s.closeChooser);
  const project = useAppStore(s => s.workspace?.project);
  const [query, setQuery] = useState('');
  const [index, setIndex] = useState(0);
  const choices = useMemo(
    () =>
      project
        ? assetChoices(
            project.assets ?? [],
            pending ?? [],
            project.id,
            stageLabels(project.target),
            query
          )
        : [],
    [project, pending, query]
  );
  const active = choices[Math.min(index, choices.length - 1)] ?? null;
  const more = askingArrivals(status).filter(a => a.id !== arrivalId).length;

  useEffect(() => {
    // A new download to place: start the search again.
    setQuery('');
    setIndex(0);
    ref.current?.querySelector('input')?.focus();
  }, [arrivalId]);

  const move = (to: number) => {
    const next = Math.max(0, Math.min(choices.length - 1, to));
    setIndex(next);
    const choice = choices[next];
    if (choice)
      document
        .getElementById(choiceDomId(listId, choice.asset))
        ?.scrollIntoView({ block: 'nearest' });
  };

  const pick = (choice: AssetChoice) => {
    if (!busy) void attach(arrivalId, choice.asset);
  };

  const onInputKey = (e: KeyboardEvent) => {
    const at = active ? choices.indexOf(active) : 0;
    if (e.key === 'ArrowDown') move(at + 1);
    else if (e.key === 'ArrowUp') move(at - 1);
    else if (e.key === 'Home') move(0);
    else if (e.key === 'End') move(choices.length - 1);
    else if (e.key === 'Enter' && active) pick(active);
    else return;
    e.preventDefault();
  };

  // Escape is the chooser's alone: the picker under it stays open.
  const onDialogKey = (e: KeyboardEvent) => {
    if (e.key !== 'Escape') return;
    e.preventDefault();
    e.stopPropagation();
    if (query) {
      setQuery('');
      setIndex(0);
    } else close();
  };

  return {
    ref,
    listId,
    arrival,
    project,
    query,
    search: (value: string) => {
      setQuery(value);
      setIndex(0);
    },
    choices,
    active,
    busy,
    more,
    pick,
    close,
    onInputKey,
    onDialogKey,
  };
}
