import { useEffect, useState } from 'react';
import { metaProblem, parseAuthors } from '@/state/avatar';
import { useAvatarStore } from '@/state/use-avatar-store';

/** The draft meta, its authors as typed, and what a build would refuse in it. */
export function useMetaForm() {
  const draft = useAvatarStore(s => s.draft);
  const stored = useAvatarStore(s => s.view?.stored ?? false);
  const asset = useAvatarStore(s => s.asset);
  const dirty = useAvatarStore(s => s.dirty);
  const saving = useAvatarStore(s => s.saving);
  const editMeta = useAvatarStore(s => s.editMeta);
  const save = useAvatarStore(s => s.save);
  const joined = draft?.meta.authors.join(', ') ?? '';
  // The authors as typed, so a trailing comma survives until the next name.
  const [authors, setAuthors] = useState(joined);
  useEffect(() => {
    if (!dirty) setAuthors(joined);
  }, [asset, joined, dirty]);
  return {
    meta: draft?.meta ?? null,
    stored,
    dirty,
    saving,
    problem: draft ? metaProblem(draft.meta) : null,
    authors,
    setAuthors: (text: string) => {
      setAuthors(text);
      editMeta({ authors: parseAuthors(text) });
    },
    editMeta,
    save: () => void save(),
  };
}
