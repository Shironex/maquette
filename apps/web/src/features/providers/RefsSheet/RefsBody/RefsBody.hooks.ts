import { useState } from 'react';
import type { AssetRef, RefsView } from '@/ipc/client';
import { emptyViews, filledCount } from '@/state/refs';
import { useRefsStore } from '@/state/use-refs-store';

/** What the refs page shows of the asset's refs, and what it can do with them. */
export function useRefsBody(view: RefsView) {
  const { tag, add, remove, show, close, busy } = useRefsStore();
  const [adding, setAdding] = useState(false);
  const refs = view.refs;
  const front = refs.find(r => r.view === 'front') ?? null;
  const free = view.images.filter(i => !refs.some(r => r.file === i.file));
  const running = view.jobs.filter(j => j.state === 'queued' || j.state === 'running').length;
  const n = refs.length;
  return {
    busy,
    front,
    others: refs.filter(r => r.view === 'other'),
    free,
    running,
    allFilled: emptyViews(refs).length === 0,
    readFromOrder: !view.tagged && n > 0,
    footer: `${filledCount(refs)} of 4 views · ${n} ${n === 1 ? 'ref' : 'refs'}`,
    adding,
    toggleAdding: () => setAdding(!adding),
    stopAdding: () => setAdding(false),
    actions: (item: AssetRef) => ({
      onTag: (next: AssetRef['view']) => void tag(item.file, next),
      onClean: () => show('clean', item.file),
      onRemove: () => void remove(item.file),
    }),
    add: (file: string) => void add([file]),
    drawViews: () => show('views'),
    close,
  };
}
