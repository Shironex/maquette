import { useState, type KeyboardEvent } from 'react';
import { MAX_OBJECT_CHARS } from '@/state/lab';

/** The draft name and notes, and Save with the reason it was refused. */
export function useObjectEditor(
  name: string,
  notes: string,
  onSave: (next: { name: string; notes: string }) => boolean
) {
  const [draftName, setDraftName] = useState(name);
  const [draftNotes, setDraftNotes] = useState(notes);
  const [error, setError] = useState<string | null>(null);
  const tooLong = draftName.trim().length > MAX_OBJECT_CHARS;
  const save = () => {
    if (onSave({ name: draftName, notes: draftNotes })) return;
    setError(
      draftName.trim() ? 'Another object already has that name.' : 'The object needs a name.'
    );
  };
  const rename = (next: string) => {
    setDraftName(next);
    setError(null);
  };
  /** Cmd+Enter anywhere in the editor saves, and stays out of the dialog's own shortcut. */
  const onGroupKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key !== 'Enter' || !(e.metaKey || e.ctrlKey) || tooLong) return;
    e.preventDefault();
    e.stopPropagation();
    save();
  };
  const onNameKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key !== 'Enter' || tooLong) return;
    e.preventDefault();
    save();
  };
  return {
    draftName,
    rename,
    draftNotes,
    setDraftNotes,
    error,
    tooLong,
    save,
    onGroupKeyDown,
    onNameKeyDown,
  };
}
