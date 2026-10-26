import { useEffect, useRef, useState, type RefObject } from 'react';
import {
  device,
  fitLabel,
  fitScale,
  goto,
  hello,
  nearestStop,
  readPageMessage,
} from '@/state/page';
import { usePageStore } from '@/state/use-page-store';
import type { PageFrameProps } from './PageFrame.types';

/** Hello is sent again this often until the page answers, for pages that load their script late. */
const HELLO_EVERY = 500;
const HELLO_TRIES = 20;

/** The size of the element, kept as it resizes. */
export function useBox<T extends HTMLElement>() {
  const ref = useRef<T>(null);
  const [box, setBox] = useState({ width: 0, height: 0 });
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const measure = () => setBox({ width: el.clientWidth, height: el.clientHeight });
    measure();
    // Without ResizeObserver (tests), a window resize still measures again.
    if (typeof ResizeObserver === 'undefined') {
      window.addEventListener('resize', measure);
      return () => window.removeEventListener('resize', measure);
    }
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);
  return { ref, box };
}

/**
 * Speaks the page bridge with the page in `frame`: says hello once it
 * loads, hears only that frame's window, and sends every seek of the
 * scrubber. Messages carry no secrets, so they go to any origin: the frame
 * shows a local dev server or a file the person picked.
 */
export function usePageBridge(
  id: string,
  frame: RefObject<HTMLIFrameElement | null>,
  src: string | null,
  /** Changes when the frame element is made again (Reload). */
  generation: number
) {
  const onMessage = usePageStore(s => s.onMessage);
  const forgetFrame = usePageStore(s => s.forgetFrame);
  const seekId = usePageStore(s => s.seekId);

  useEffect(() => {
    const el = frame.current;
    if (!el || !src) return;
    let timer: ReturnType<typeof setInterval> | undefined;
    const post = (message: unknown) => el.contentWindow?.postMessage(message, '*');
    const onLoad = () => {
      clearInterval(timer);
      forgetFrame(id);
      let tries = 0;
      post(hello());
      timer = setInterval(() => {
        tries += 1;
        if (tries >= HELLO_TRIES) clearInterval(timer);
        else post(hello());
      }, HELLO_EVERY);
    };
    const onWindowMessage = (event: MessageEvent) => {
      if (event.source !== el.contentWindow) return;
      const message = readPageMessage(event.data);
      if (!message) return;
      if (message.type === 'ready') {
        clearInterval(timer);
        // A page that loads again (a reload, another device) meets the scrubber where it is.
        const { target } = usePageStore.getState();
        if (target > 0) post(goto(target));
      }
      onMessage(id, message);
    };
    el.addEventListener('load', onLoad);
    window.addEventListener('message', onWindowMessage);
    // The page may have loaded before this ran; the retries cover a slow one.
    onLoad();
    return () => {
      clearInterval(timer);
      el.removeEventListener('load', onLoad);
      window.removeEventListener('message', onWindowMessage);
      forgetFrame(id);
    };
  }, [id, frame, src, generation, onMessage, forgetFrame]);

  useEffect(() => {
    if (seekId === 0) return;
    const { target, smooth } = usePageStore.getState();
    frame.current?.contentWindow?.postMessage(goto(target, smooth), '*');
  }, [seekId, frame]);
}

/** The frame's scale, its badge line, the stop pin and Reload. */
export function usePageFrame({ id, src, device: deviceId, fit = 'contain' }: PageFrameProps) {
  const frame = useRef<HTMLIFrameElement>(null);
  const viewport = useBox<HTMLDivElement>();
  const reloadKey = usePageStore(s => s.reloadKey);
  usePageBridge(id, frame, src, reloadKey);
  const reload = usePageStore(s => s.reload);
  const reading = usePageStore(s => s.readings[id]);
  const d = device(deviceId);
  const scale =
    fit === 'width' ? Math.min(1, viewport.box.width / d.width) : fitScale(d, viewport.box);
  const stops = reading?.stops ?? [];
  const at = reading?.at ?? null;
  const index = at === null ? null : nearestStop(at, stops.length);
  return {
    frame,
    viewport: viewport.ref,
    d,
    scale,
    fitText: fitLabel(d, scale),
    reloadKey,
    reload,
    pin: index === null || !stops[index] ? null : { number: index + 1, name: stops[index].name },
  };
}
