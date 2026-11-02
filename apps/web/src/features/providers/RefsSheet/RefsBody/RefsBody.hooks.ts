import { useState } from 'react';
import { commands, errorMessage, type AssetRef, type RefsView } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
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
    /** Copies images from anywhere into `refs/`. A card with no front gets the first as its front. */
    addFromDisk: async () => {
      setAdding(false);
      const picked = await commands.dialogPickFiles('Choose reference images');
      if (picked.length === 0) return;
      try {
        const files = await commands.labImportRefs(view.project.id, picked);
        // An import can answer with a ref already here, which keeps its view.
        const fresh = files.filter(file => !refs.some(r => r.file === file));
        if (fresh.length > 0) await add(fresh);
        if (!front && files[0]) await tag(files[0], 'front');
      } catch (error) {
        useAppStore.getState().notify({
          status: 'failed',
          title: 'Could not add those images',
          detail: errorMessage(error),
        });
      }
    },
    drawViews: () => show('views'),
    close,
  };
}
