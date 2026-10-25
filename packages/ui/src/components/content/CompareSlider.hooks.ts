import { useRef, useState, type KeyboardEvent } from 'react';

/** The divider position in percent, moved by pointer or the arrow keys. */
export function useCompareSlider(initial: number) {
  const [pos, setPos] = useState(initial);
  const ref = useRef<HTMLDivElement>(null);
  function move(clientX: number) {
    const r = ref.current?.getBoundingClientRect();
    if (!r || r.width === 0) return;
    setPos(Math.max(0, Math.min(100, ((clientX - r.left) / r.width) * 100)));
  }
  function key(e: KeyboardEvent) {
    if (e.key === 'ArrowLeft') setPos(p => Math.max(0, p - 5));
    if (e.key === 'ArrowRight') setPos(p => Math.min(100, p + 5));
  }
  return { pos, ref, move, key };
}
