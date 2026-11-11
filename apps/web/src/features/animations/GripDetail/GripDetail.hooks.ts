import { useState } from 'react';
import { commands, errorMessage, type LibraryGripDoc, type LibraryGripSummary } from '@/ipc/client';
import { useAsyncValue } from '@/lib/useAsyncValue';
import { newestFirst } from '@/state/recipe-library';
import { useAppStore } from '@/state/use-app-store';
import { useGripLibraryStore } from '@/state/use-grip-library-store';

interface RevisionRead {
  entry: string;
  revision: number;
  /** The entry's version, so a save or removal reads it again. */
  version: string;
}

type Read = { doc: LibraryGripDoc; error: null } | { doc: null; error: string };

const readRevision = ({ entry, revision }: RevisionRead): Promise<Read> =>
  commands.gripLibraryGet(entry, revision).then(
    doc => ({ doc, error: null }),
    error => ({ doc: null, error: errorMessage(error) })
  );

/** The two removals the panel asks about first. */
export type Asking = 'revision' | 'grip' | null;

/**
 * The picked revision (the newest until another is picked, and again once
 * the picked one is removed) read from the library, and the removals.
 */
export function useGripDetail(summary: LibraryGripSummary) {
  const projects = useAppStore(s => s.workspace?.projects);
  const [picked, setPicked] = useState(summary.latest);
  const [asking, setAsking] = useState<Asking>(null);
  const [removing, setRemoving] = useState(false);
  const [refused, setRefused] = useState<string | null>(null);
  const revision = summary.revisions.some(r => r.revision === picked) ? picked : summary.latest;
  const read = useAsyncValue(
    readRevision,
    summary.latest > 0 ? { entry: summary.id, revision, version: summary.version } : null,
    0
  );
  const current = read?.doc?.revision === revision ? read : null;
  const store = useGripLibraryStore.getState();
  const remove = async () => {
    setRemoving(true);
    const answer =
      asking === 'grip'
        ? await store.remove(summary.id)
        : await store.removeRevision(summary.id, revision);
    setRemoving(false);
    setAsking(null);
    setRefused(answer);
  };
  return {
    revisions: newestFirst(summary.revisions),
    revision,
    doc: current?.doc ?? null,
    error: read?.error ?? null,
    projectName: (id: string) => projects?.find(p => p.id === id)?.name ?? id,
    pick: (n: number) => {
      setPicked(n);
      setRefused(null);
    },
    asking,
    ask: (what: Asking) => {
      setRefused(null);
      setAsking(what);
    },
    removing,
    refused,
    remove: () => void remove(),
  };
}
