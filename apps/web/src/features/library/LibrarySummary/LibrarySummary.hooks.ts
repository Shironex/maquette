import { useState } from 'react';

/** Whether the sort menu is open. */
export function useSortMenu() {
  const [sorting, setSorting] = useState(false);
  return { sorting, toggle: () => setSorting(s => !s), close: () => setSorting(false) };
}
