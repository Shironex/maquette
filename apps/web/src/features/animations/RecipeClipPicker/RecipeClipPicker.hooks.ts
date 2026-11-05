import { useEffect, useMemo, useState } from 'react';
import { pickableClips } from '@/state/recipes';
import { useAnimationsStore } from '@/state/use-animations-store';
import { useRecipesStore } from '@/state/use-recipes-store';

/** The base clip picker: the library's clips, a search, the chosen clip and the recipe name. */
export function useRecipeClipPicker() {
  const picking = useRecipesStore(s => s.picking);
  const creating = useRecipesStore(s => s.creating);
  const error = useRecipesStore(s => s.pickError);
  const view = useAnimationsStore(s => s.view);
  const libraryError = useAnimationsStore(s => s.error);
  const [query, setQuery] = useState('');
  const [chosen, setChosen] = useState<string | null>(null);
  const [name, setName] = useState('');
  useEffect(() => {
    if (picking && !useAnimationsStore.getState().view) void useAnimationsStore.getState().load();
  }, [picking]);
  useEffect(() => {
    if (picking) return;
    setQuery('');
    setChosen(null);
    setName('');
  }, [picking]);
  const items = useMemo(() => view?.items ?? [], [view]);
  const shown = useMemo(() => pickableClips(items, query), [items, query]);
  const clip = items.find(i => i.clip.id === chosen)?.clip ?? null;
  const { closePicker, create } = useRecipesStore.getState();
  return {
    open: picking !== null,
    loading: !view && !libraryError,
    libraryError,
    error,
    creating,
    total: items.length,
    shown,
    query,
    setQuery,
    clip,
    choose: (id: string) => {
      const next = items.find(i => i.clip.id === id)?.clip;
      // The name follows the clip until the person types their own.
      if (next && (!name || name === clip?.name)) setName(next.name);
      setChosen(id);
    },
    name,
    setName,
    close: closePicker,
    make: () => (clip ? void create(clip.id, name) : undefined),
  };
}
