import { useState } from 'react';

/** Whether the sort menu and the group menu are open; opening one closes the other. */
export function useSortMenu() {
  const [open, setOpen] = useState<'sort' | 'group' | null>(null);
  const flip = (menu: 'sort' | 'group') => () => setOpen(now => (now === menu ? null : menu));
  return {
    sorting: open === 'sort',
    grouping: open === 'group',
    toggle: flip('sort'),
    toggleGroup: flip('group'),
    close: () => setOpen(null),
  };
}
