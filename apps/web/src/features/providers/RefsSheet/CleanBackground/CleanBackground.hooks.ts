import { useState } from 'react';
import type { CleanGround, RefsView } from '@/ipc/client';
import { candidates } from '@/state/refs';
import { useRefsStore } from '@/state/use-refs-store';

/** The copy's options, the newest clean copy of `file` and the actions on it. */
export function useCleanBackground(view: RefsView, file: string) {
  const { clean, use, show, busy } = useRefsStore();
  const [ground, setGround] = useState<CleanGround>('studio-grey');
  const [pad, setPad] = useState(false);
  const [extend, setExtend] = useState(false);
  const copy =
    candidates(view.directions, 'clean')
      .filter(c => c.forWhat === file)
      .at(-1) ?? null;
  const drawing = view.jobs.some(
    j => j.title === file && (j.state === 'queued' || j.state === 'running')
  );
  return {
    busy,
    drawing,
    copy,
    tag: view.refs.find(r => r.file === file)?.view ?? 'other',
    ground,
    setGround,
    pad,
    setPad,
    extend,
    setExtend,
    draw: () => void clean(file, ground, pad, extend),
    useCopy: (copyFile: string) => void use(copyFile).then(() => show('refs')),
    keepOriginal: () => show('refs'),
  };
}
