import { useState } from 'react';
import type { AssetRef, RefView } from '@/ipc/client';
import { SLOTS, VIEW_LABEL } from '@/state/refs';
import type { MenuItem } from '@/features/common/Menu';

export const nameOf = (file: string) => file.slice(file.lastIndexOf('/') + 1);

/**
 * The chip's menu: every view, the one a pick swaps with named in its
 * hint, then Clean background and Leave out.
 */
export function useViewTile(
  item: AssetRef,
  refs: readonly AssetRef[],
  actions: { onTag: (view: RefView) => void; onClean: () => void; onRemove: () => void }
) {
  const [open, setOpen] = useState(false);
  const hintFor = (view: RefView) => {
    if (view === 'other') return 'Kept with the asset, never sent as a view';
    const holder = refs.find(r => r.view === view && r.file !== item.file);
    return holder
      ? `Swaps the two: ${nameOf(holder.file)} becomes ${VIEW_LABEL[item.view]}`
      : undefined;
  };
  const views: MenuItem[] = [...SLOTS, 'other' as const].map(view => ({
    id: view,
    label: view === item.view ? `${VIEW_LABEL[view]} (now)` : VIEW_LABEL[view],
    icon: view === item.view ? 'check' : undefined,
    section: 'views',
    disabled: view === item.view,
    hint: hintFor(view),
    onSelect: () => actions.onTag(view),
  }));
  const items: MenuItem[] = [
    ...views,
    {
      id: 'clean',
      label: 'Clean background...',
      icon: 'magic-wand',
      section: 'more',
      onSelect: actions.onClean,
    },
    {
      id: 'remove',
      label: 'Leave out of the refs',
      icon: 'x',
      section: 'more',
      hint: 'The image stays in refs/',
      onSelect: actions.onRemove,
    },
  ];
  return { open, toggle: () => setOpen(!open), close: () => setOpen(false), items };
}
