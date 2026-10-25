import { useRef } from 'react';

/** The handle's box, and the split for a pointer at `clientX` across its parent. */
export function useSplitDrag(onChange: (split: number) => void) {
  const drag = useRef<HTMLDivElement>(null);
  const move = (clientX: number) => {
    const box = drag.current?.parentElement?.getBoundingClientRect();
    if (!box || box.width === 0) return;
    onChange(Math.min(0.98, Math.max(0.02, (clientX - box.left) / box.width)));
  };
  return { drag, move };
}
