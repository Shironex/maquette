/**
 * Opens the mock workspace with planning canvases (`?plans=demo` in the
 * browser, or none), loads Frostveil's list into the Plan store, and gives
 * React Flow the browser parts jsdom lacks.
 */
import { usePlansStore } from '@/state/use-plans-store';
import { openMockWorkspace } from './mock-ipc';

class FakeResizeObserver {
  constructor(private readonly callback: ResizeObserverCallback) {}
  observe(target: Element) {
    const entry = { target, contentRect: { width: 1200, height: 700 } } as ResizeObserverEntry;
    this.callback([entry], this as unknown as ResizeObserver);
  }
  unobserve() {}
  disconnect() {}
}

class FakeDomMatrix {
  m22: number;
  constructor(transform?: string) {
    const scale = /scale\(([\d.]+)\)/.exec(transform ?? '');
    this.m22 = scale ? Number(scale[1]) : 1;
  }
}

/** React Flow measures its pane and reads transforms; jsdom lays nothing out. */
export function fakeFlowLayout() {
  globalThis.ResizeObserver ??= FakeResizeObserver as unknown as typeof ResizeObserver;
  (globalThis as { DOMMatrixReadOnly?: unknown }).DOMMatrixReadOnly ??= FakeDomMatrix;
  // Edge labels measure their text.
  const svg = SVGElement.prototype as SVGElement & { getBBox?: () => DOMRect };
  svg.getBBox ??= () => ({ x: 0, y: 0, width: 40, height: 14 }) as DOMRect;
  for (const [key, value] of [
    ['offsetWidth', 200],
    ['offsetHeight', 120],
  ] as const) {
    if (!Object.getOwnPropertyDescriptor(HTMLElement.prototype, key)?.configurable) continue;
    Object.defineProperty(HTMLElement.prototype, key, { configurable: true, get: () => value });
  }
}

/** `demo` gives Frostveil four canvases; anything else none. */
export async function openPlans(plans: string | null = 'demo') {
  usePlansStore.setState(usePlansStore.getInitialState(), true);
  await openMockWorkspace(
    'configured',
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    plans ?? undefined
  );
  await usePlansStore.getState().loadList('frostveil');
  return usePlansStore.getState();
}

export { clearMocks } from './mock-ipc';
