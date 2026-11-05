import { useState } from 'react';
import {
  commands,
  errorMessage,
  type LibraryRecipeDoc,
  type LibraryRecipeSummary,
} from '@/ipc/client';
import { useAsyncValue } from '@/lib/useAsyncValue';
import { newestFirst } from '@/state/recipe-library';
import { useAppStore } from '@/state/use-app-store';
import { useRecipeLibraryStore } from '@/state/use-recipe-library-store';

interface RevisionRead {
  entry: string;
  revision: number;
  /** The entry's version, so a save or removal reads it again. */
  version: string;
}

type Read = { doc: LibraryRecipeDoc; error: null } | { doc: null; error: string };

const readRevision = ({ entry, revision }: RevisionRead): Promise<Read> =>
  commands.recipeLibraryGet(entry, revision).then(
    doc => ({ doc, error: null }),
    error => ({ doc: null, error: errorMessage(error) })
  );

/** The two removals the panel asks about first. */
export type Asking = 'revision' | 'recipe' | null;

/**
 * The picked revision (the newest until another is picked, and again once
 * the picked one is removed) read from the library, and the panel's actions.
 */
export function useLibraryRecipeDetail(summary: LibraryRecipeSummary) {
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
  const store = useRecipeLibraryStore.getState();
  const remove = async () => {
    setRemoving(true);
    const answer =
      asking === 'recipe'
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
    use: () => store.openUse({ entry: summary.id, revision, name: summary.name }),
    exportFile: () =>
      void store.exportRecipe(
        { project: null, recipe: null, entry: summary.id, revision },
        summary.name
      ),
  };
}
