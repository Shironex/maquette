import { useRef, useState, type KeyboardEvent, type PointerEvent } from 'react';
import { readJson, writeJson } from '@/lib/storage';
import { SIDE_WIDTH, clampWidth, keyWidth } from './sideWidth';

const KEY = 'maquette.pipeline.sideWidth';

function stored(): number {
  const value = readJson(KEY);
  return typeof value === 'number' ? clampWidth(value) : SIDE_WIDTH.initial;
}

/** The side panel's width, remembered for this viewer. */
export function useSideWidth() {
  const [width, setWidth] = useState(stored);
  const change = (next: number) => {
    setWidth(next);
    writeJson(KEY, next);
  };
  return { width, setWidth: change, reset: () => change(SIDE_WIDTH.initial) };
}

/** Dragging the handle and its keys; the row it sits in sets the room. */
export function useResizeDrag(width: number, onChange: (width: number) => void) {
  const start = useRef<{ x: number; width: number; room: number } | null>(null);
  const [dragging, setDragging] = useState(false);
  const room = (el: HTMLElement) => el.closest('[data-resize-row]')?.getBoundingClientRect().width;
  return {
    dragging,
    onPointerDown: (e: PointerEvent<HTMLElement>) => {
      e.currentTarget.setPointerCapture(e.pointerId);
      start.current = { x: e.clientX, width, room: room(e.currentTarget) ?? 0 };
      setDragging(true);
    },
    onPointerMove: (e: PointerEvent<HTMLElement>) => {
      const s = start.current;
      if (s) onChange(clampWidth(s.width + s.x - e.clientX, s.room || undefined));
    },
    onPointerUp: () => {
      start.current = null;
      setDragging(false);
    },
    onKeyDown: (e: KeyboardEvent<HTMLElement>) => {
      const next = keyWidth(e.key, width);
      if (next === null) return;
      e.preventDefault();
      onChange(clampWidth(next, room(e.currentTarget)));
    },
  };
}
