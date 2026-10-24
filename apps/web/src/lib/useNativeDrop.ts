import { useEffect, useRef } from 'react';
import { IN_TAURI } from '@/ipc/runtime';

/** A point in CSS pixels of the web view. */
export interface DropPoint {
  x: number;
  y: number;
}

export interface NativeDropHandlers {
  onOver?: (at: DropPoint) => void;
  onLeave?: () => void;
  onDrop: (at: DropPoint, paths: string[]) => void;
}

/**
 * In the app, files dropped on the window arrive as a Tauri event with
 * their paths (the web view never sees them). The handlers get the point
 * in CSS pixels, so a caller can tell which element the drop is over. A
 * plain browser has no such event, so this does nothing there.
 */
export function useNativeDrop(handlers: NativeDropHandlers) {
  const latest = useRef(handlers);
  latest.current = handlers;
  useEffect(() => {
    if (!IN_TAURI) return;
    let stop: (() => void) | null = null;
    let cancelled = false;
    const point = (x: number, y: number): DropPoint => {
      const ratio = window.devicePixelRatio || 1;
      return { x: x / ratio, y: y / ratio };
    };
    void import('@tauri-apps/api/webview').then(({ getCurrentWebview }) =>
      getCurrentWebview()
        .onDragDropEvent(event => {
          const p = event.payload;
          if (p.type === 'over') latest.current.onOver?.(point(p.position.x, p.position.y));
          else if (p.type === 'leave') latest.current.onLeave?.();
          else if (p.type === 'drop') {
            latest.current.onDrop(point(p.position.x, p.position.y), p.paths);
          }
        })
        .then(fn => {
          if (cancelled) fn();
          else stop = fn;
        })
    );
    return () => {
      cancelled = true;
      stop?.();
    };
  }, []);
}
