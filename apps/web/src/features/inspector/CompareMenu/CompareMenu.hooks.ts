import { useState } from 'react';
import type { InspectView } from '@/ipc/client';
import type { MenuItem } from '@/features/common/Menu';
import type { SidePick } from '@/state/inspector';
import { useInspectorStore } from '@/state/use-inspector-store';

/**
 * Whether the menu is open, and what it offers: every comparison, then
 * each other version of this one to set against the current.
 */
export function useCompareMenu(view: InspectView, pick: SidePick) {
  const [open, setOpen] = useState(false);
  const choose = useInspectorStore(s => s.pick);
  const items: MenuItem[] = [
    ...view.comparisons.map(c => ({
      id: `c-${c.id}`,
      label: c.label === 'Scene' ? 'Scene' : `${c.label}, versions`,
      icon: (c.kind === 'scene' ? 'squares-four' : 'cube') as 'cube',
      disabled: c.id === pick.comparison.id,
      onSelect: () => choose({ comparison: c.id, left: null, right: null }),
    })),
    ...pick.comparison.sides
      .filter(s => s.id !== pick.right.id && s.id !== pick.left?.id)
      .map(s => ({
        id: `l-${s.id}`,
        label: `Compare ${s.label} with ${pick.right.label}`,
        icon: 'arrows-left-right' as const,
        onSelect: () => choose({ left: s.id }),
      })),
  ];
  return { open, toggle: () => setOpen(m => !m), close: () => setOpen(false), items };
}
