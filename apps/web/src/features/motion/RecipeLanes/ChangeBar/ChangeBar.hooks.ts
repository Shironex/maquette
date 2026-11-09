import { useRef, type KeyboardEvent, type PointerEvent } from 'react';
import type { RecipeLayer } from '@/ipc/client';
import { num } from '@/state/recipe-draft';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';

/** What a drag moves: the whole bar, or one of its ends. */
export type BarPart = 'move' | 'start' | 'end';

interface Drag {
  part: BarPart;
  x: number;
  width: number;
  from: number;
  to: number;
}

const KEY_STEP: Record<string, number> = { ArrowLeft: -1, ArrowRight: 1 };

/**
 * `from` and `to` (bake frames) after moving `part` by `delta` frames on a
 * ruler of `frames`: a move keeps the length, an end stays a frame clear of
 * the other.
 */
export function moved(part: BarPart, from: number, to: number, delta: number, frames: number) {
  if (part === 'move') {
    const shift = Math.min(Math.max(delta, -from), frames - to);
    return [from + shift, to + shift] as const;
  }
  if (part === 'start') return [Math.min(Math.max(from + delta, 0), to - 1), to] as const;
  return [from, Math.max(Math.min(to + delta, frames), from + 1)] as const;
}

/**
 * A change's bar on the bake frames: drag it to move it, drag its ends to
 * change the range, or use the arrow keys a frame at a time (Shift for 10).
 * Frames turn into the change's shares through the timeline.
 */
export function useChangeBar(
  layer: RecipeLayer,
  frames: number,
  toFrame: (share: number) => number,
  toShare: (frame: number) => number
) {
  const drag = useRef<Drag | null>(null);
  const from = toFrame(num(layer.settings.start));
  const to = toFrame(num(layer.settings.end, 1));
  const put = (next: readonly [number, number]) => {
    const [a, b] = next;
    if (a === from && b === to) return;
    useRecipeEditorStore.getState().edit(d => ({
      ...d,
      layers: d.layers.map(l =>
        l.id === layer.id
          ? { ...l, settings: { ...l.settings, start: toShare(a), end: toShare(b) } }
          : l
      ),
    }));
  };
  const down = (part: BarPart) => (e: PointerEvent<HTMLElement>) => {
    e.stopPropagation();
    const lane = e.currentTarget.closest('[data-lane]');
    const width = lane?.getBoundingClientRect().width ?? 0;
    e.currentTarget.setPointerCapture?.(e.pointerId);
    drag.current = { part, x: e.clientX, width, from, to };
  };
  const move = (e: PointerEvent<HTMLElement>) => {
    const d = drag.current;
    if (!d || e.buttons !== 1 || d.width <= 0) return;
    const delta = Math.round(((e.clientX - d.x) / d.width) * frames);
    put(moved(d.part, d.from, d.to, delta, frames));
  };
  const up = () => {
    drag.current = null;
  };
  const key = (part: BarPart) => (e: KeyboardEvent<HTMLElement>) => {
    const step = KEY_STEP[e.key];
    if (!step) return;
    e.preventDefault();
    e.stopPropagation();
    put(moved(part, from, to, step * (e.shiftKey ? 10 : 1), frames));
  };
  return { from, to, down, move, up, key };
}
