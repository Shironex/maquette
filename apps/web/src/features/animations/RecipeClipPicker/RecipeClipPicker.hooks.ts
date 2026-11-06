import { useEffect, useMemo, useState } from 'react';
import { addClip, clipOfLibrary } from '@/state/recipe-clips';
import { pickableClips } from '@/state/recipes';
import { useAnimationsStore } from '@/state/use-animations-store';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { useRecipesStore } from '@/state/use-recipes-store';

/** What a recipe on the rest pose is called until the person names it. */
export const REST_POSE = 'Rest pose';

/**
 * The base clip picker: the library's clips, a search, the chosen clip (or
 * the rest pose, no clip) and the recipe name. Joining, it adds the chosen
 * clip to the end of the recipe open in the editor instead.
 */
export function useRecipeClipPicker() {
  const picking = useRecipesStore(s => s.picking);
  const joining = useRecipesStore(s => s.joining);
  const creating = useRecipesStore(s => s.creating);
  const error = useRecipesStore(s => s.pickError);
  const view = useAnimationsStore(s => s.view);
  const libraryError = useAnimationsStore(s => s.error);
  const [query, setQuery] = useState('');
  const [chosen, setChosen] = useState<string | null>(null);
  const [rest, setRest] = useState(false);
  const [name, setName] = useState('');
  useEffect(() => {
    if (picking && !useAnimationsStore.getState().view) void useAnimationsStore.getState().load();
  }, [picking]);
  useEffect(() => {
    if (picking) return;
    setQuery('');
    setChosen(null);
    setRest(false);
    setName('');
  }, [picking]);
  const items = useMemo(() => view?.items ?? [], [view]);
  const shown = useMemo(() => pickableClips(items, query), [items, query]);
  const clip = rest ? null : (items.find(i => i.clip.id === chosen)?.clip ?? null);
  const following = !name || name === (rest ? REST_POSE : clip?.name);
  const { closePicker, create } = useRecipesStore.getState();
  return {
    open: picking !== null,
    joining,
    loading: !view && !libraryError,
    libraryError,
    error,
    creating,
    total: items.length,
    shown,
    query,
    setQuery,
    clip,
    rest,
    picked: rest || clip !== null,
    choose: (id: string) => {
      const next = items.find(i => i.clip.id === id)?.clip;
      // The name follows the clip until the person types their own.
      if (next && following) setName(next.name);
      setChosen(id);
      setRest(false);
    },
    chooseRest: () => {
      if (following) setName(REST_POSE);
      setRest(true);
      setChosen(null);
    },
    name,
    setName,
    close: closePicker,
    join: () => {
      if (!clip) return;
      const editor = useRecipeEditorStore.getState();
      editor.edit(d => addClip(d, clipOfLibrary(clip)));
      editor.pick({ what: 'clips', id: '' });
      closePicker();
    },
    make: () => {
      if (rest) void create(null, name);
      else if (clip) void create(clip.id, name);
    },
  };
}
