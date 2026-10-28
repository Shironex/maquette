import { useRef, type KeyboardEvent } from 'react';
import type { LabView } from '@/ipc/client';
import { useTopEscape } from '@/lib/useTopEscape';
import { MAX_STYLE_IMAGES } from '@/state/lab-style';
import { useLabStore, type StyleDraft } from '@/state/use-lab-store';

const oneLine = (text: string) => text.split(/\s+/).filter(Boolean).join(' ');

/** True when the draft differs from the saved board. */
export function draftChanged(draft: StyleDraft, view: LabView): boolean {
  const board = view.style;
  return (
    oneLine(draft.line) !== board.line ||
    draft.palette.join() !== board.palette.join() ||
    draft.images.join('\n') !== board.images.map(i => i.file).join('\n')
  );
}

/** The dialog's draft edits, Save on the submit chord, and Escape to close. */
export function useStyleBoardSheet(draft: StyleDraft, view: LabView) {
  const setStyle = useLabStore(s => s.setStyle);
  const close = useLabStore(s => s.closeStyle);
  const save = useLabStore(s => s.saveStyle);
  const busy = useLabStore(s => s.busy === 'style');
  const ref = useRef<HTMLDivElement>(null);
  useTopEscape(ref, close);

  const changed = draftChanged(draft, view);
  const empty = draft.line.trim() === '' && draft.palette.length === 0 && draft.images.length === 0;
  const addImage = (file: string) => {
    if (draft.images.includes(file) || draft.images.length >= MAX_STYLE_IMAGES) return;
    setStyle({ images: [...draft.images, file] });
  };
  const removeImage = (file: string) => setStyle({ images: draft.images.filter(f => f !== file) });
  const clear = () => setStyle({ line: '', palette: [], images: [] });
  const submit = () => {
    if (changed && !busy) void save();
  };
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key !== 'Enter' || !(e.metaKey || e.ctrlKey)) return;
    e.preventDefault();
    submit();
  };

  return {
    setStyle,
    close,
    busy,
    ref,
    changed,
    empty,
    addImage,
    removeImage,
    clear,
    submit,
    onKeyDown,
  };
}
