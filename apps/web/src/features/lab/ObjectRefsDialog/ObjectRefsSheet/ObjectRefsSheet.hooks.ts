import { useRef, useState, type ClipboardEvent, type KeyboardEvent } from 'react';
import { useDismissedTip } from '@/lib/useDismissedTip';
import { useTopEscape } from '@/lib/useTopEscape';
import { useAppStore } from '@/state/use-app-store';
import { MAX_OBJECT_CHARS, addObjects, editObject, longestObject } from '@/state/lab';
import { useLabStore, type ObjectsDraft } from '@/state/use-lab-store';

/** The id the notes tip is remembered under. */
const NOTES_TIP = 'object-notes';

/** Whether the tip about object notes was dismissed, remembered across opens. */
export function useNotesTip() {
  return useDismissedTip(NOTES_TIP);
}

/** The object field: the typed text, the chips, the one being edited, and Make. */
export function useObjectRefsSheet(objects: ObjectsDraft) {
  const setObjects = useLabStore(s => s.setObjects);
  const close = useLabStore(s => s.closeObjects);
  const make = useLabStore(s => s.makeObjects);
  const busy = useLabStore(s => s.busy === 'objects');
  const setView = useAppStore(s => s.setView);
  // The project style shows only with the Lab of the same project loaded.
  const labView = useLabStore(s => (s.view?.project === objects.project ? s.view : null));
  const [text, setText] = useState('');
  const [editing, setEditing] = useState<string | null>(null);
  const ref = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);
  useTopEscape(ref, close);

  const list = objects.objects;
  // A name is never cut: one over the limit stays in the field, with a counter.
  const longest = longestObject(text);
  const over = longest > MAX_OBJECT_CHARS;
  const commit = (value: string) => {
    if (longestObject(value) > MAX_OBJECT_CHARS) {
      setText(value);
      return;
    }
    const next = addObjects(list, value);
    if (next.length !== list.length) setObjects({ objects: next });
    setText('');
  };
  const remove = (name: string) => {
    setObjects({
      objects: list.filter(o => o !== name),
      notes: Object.fromEntries(Object.entries(objects.notes).filter(([key]) => key !== name)),
    });
    if (editing === name) setEditing(null);
  };
  const onKey = (e: KeyboardEvent<HTMLInputElement>) => {
    if ((e.key === 'Enter' || e.key === ',') && text.trim()) {
      e.preventDefault();
      commit(text);
    } else if (e.key === 'Enter' && (e.metaKey || e.ctrlKey) && list.length > 0 && !over) {
      e.preventDefault();
      void submit();
    } else if (e.key === 'Backspace' && !text && list.length > 0) {
      remove(list[list.length - 1] ?? '');
    }
  };
  const onPaste = (e: ClipboardEvent<HTMLInputElement>) => {
    const pasted = e.clipboardData.getData('text');
    if (!/[\n,;]/.test(pasted)) return;
    e.preventDefault();
    commit(`${text}${pasted}`);
  };
  // Text still in the field counts, so Make never drops the last object.
  const pending = addObjects(list, text);
  const submit = async () => {
    if (over) return;
    if (pending.length !== list.length) setObjects({ objects: pending });
    setText('');
    if (await make()) setView('lab');
  };
  const saveEdit = (name: string, next: { name: string; notes: string }) => {
    const edited = editObject(list, objects.notes, name, next);
    if (!edited) return false;
    setObjects(edited);
    setEditing(null);
    return true;
  };

  return {
    setObjects,
    close,
    busy,
    ref,
    input,
    text,
    setText,
    editing,
    setEditing,
    longest,
    over,
    pending,
    commit,
    remove,
    onKey,
    onPaste,
    submit,
    saveEdit,
    labView,
  };
}
