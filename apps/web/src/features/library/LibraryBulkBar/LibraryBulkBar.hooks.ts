import { useState } from 'react';
import type { MenuItem } from '@/features/common/Menu';
import type { LibraryCollection } from '@/ipc/client';
import { changeCollection, copyIntoProject } from '@/state/library-actions';
import { useAppStore } from '@/state/use-app-store';
import { usePackStore } from '@/state/use-pack-store';
import { useRenderStore } from '@/state/use-render-store';

const NONE: never[] = [];

export type BulkMenu = 'tag' | 'collection' | 'project' | null;

/** Which of the bar's menus is open, and what the collection and project menus offer. */
export function useLibraryBulkBar(
  ids: readonly string[],
  collections: readonly LibraryCollection[]
) {
  const projects = useAppStore(s => s.workspace?.projects) ?? NONE;
  const [open, setOpen] = useState<BulkMenu>(null);
  const n = ids.length;
  const collectionItems: MenuItem[] = collections.map(c => {
    const missing = ids.filter(id => !c.entries.includes(id));
    return {
      id: c.id,
      label: c.name,
      icon: 'folder-plus',
      disabled: missing.length === 0,
      hint: missing.length === 0 ? `All ${n} are in ${c.name} already` : undefined,
      onSelect: () => void changeCollection({ op: 'add', collection: c.id, entries: missing }),
    };
  });
  const projectItems: MenuItem[] = projects.map(p => ({
    id: p.id,
    label: p.name,
    icon: 'copy',
    onSelect: () => void copyIntoProject(ids, p.id),
  }));
  return {
    open,
    close: () => setOpen(null),
    toggle: (which: BulkMenu) => setOpen(o => (o === which ? null : which)),
    collectionItems,
    projectItems,
    exportPack: () => void usePackStore.getState().show({ from: 'entries', entries: [...ids] }),
    renderModels: () =>
      void useRenderStore
        .getState()
        .show({ from: 'entries', entries: [...ids] }, [...ids], 'Opened from the Models selection'),
  };
}
