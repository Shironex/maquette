import { useRef, useState, type KeyboardEvent, type PointerEvent } from 'react';
import { usePoseKeyEditing, type KeyMark } from '@/state/use-pose-key-editing';

interface Drag {
  mark: KeyMark;
  x: number;
  width: number;
  moved: boolean;
}

/** The bake frame a drag of `dx` pixels across a lane `width` wide lands on, from `from`. */
export function draggedTo(from: number, dx: number, width: number, frames: number): number {
  if (width <= 0) return from;
  return Math.min(Math.max(Math.round(from + (dx / width) * frames), 0), frames);
}

/**
 * Key diamonds on one lane: all the change's keys, or one bone's. Click a
 * diamond to go to its frame; drag it to move the key (Alt to copy it),
 * or use the arrow keys a frame at a time (Alt copies there too).
 */
export function usePoseKeyLane(id: string, bone: string | null, frames: number) {
  const e = usePoseKeyEditing(id);
  const drag = useRef<Drag | null>(null);
  const [ghost, setGhost] = useState<{ from: KeyMark; to: number } | null>(null);
  const marks = e.marks.filter(m => m.frame !== null && (!bone || m.bones.includes(bone)));
  const bones = bone ? [bone] : undefined;

  const down = (mark: KeyMark) => (ev: PointerEvent<HTMLElement>) => {
    ev.stopPropagation();
    const lane = ev.currentTarget.parentElement;
    drag.current = {
      mark,
      x: ev.clientX,
      width: lane?.getBoundingClientRect().width ?? 0,
      moved: false,
    };
    ev.currentTarget.setPointerCapture?.(ev.pointerId);
  };
  const move = (ev: PointerEvent<HTMLElement>) => {
    const d = drag.current;
    if (!d || d.mark.frame === null) return;
    const to = draggedTo(d.mark.frame, ev.clientX - d.x, d.width, frames);
    if (to !== d.mark.frame) d.moved = true;
    if (d.moved) setGhost({ from: d.mark, to });
  };
  const up = (ev: PointerEvent<HTMLElement>) => {
    const d = drag.current;
    drag.current = null;
    setGhost(null);
    if (!d || d.mark.frame === null) return;
    ev.currentTarget.releasePointerCapture?.(ev.pointerId);
    const to = draggedTo(d.mark.frame, ev.clientX - d.x, d.width, frames);
    if (d.moved && to !== d.mark.frame) e.move(d.mark.spot, to, { copy: ev.altKey, bones });
    else e.seek(d.mark.frame);
  };
  const key = (mark: KeyMark) => (ev: KeyboardEvent<HTMLElement>) => {
    if (mark.frame === null) return;
    if (ev.key === 'Enter' || ev.key === ' ') {
      ev.preventDefault();
      e.seek(mark.frame);
      return;
    }
    const step = { ArrowLeft: -1, ArrowRight: 1 }[ev.key];
    if (!step) return;
    ev.preventDefault();
    const to = Math.min(Math.max(mark.frame + step * (ev.shiftKey ? 10 : 1), 0), frames);
    e.move(mark.spot, to, { copy: ev.altKey, bones });
  };
  return { marks, ghost, frame: e.frame, down, move, up, key };
}
