/**
 * Opens the mock workspace with planning canvases (`?plans=demo` in the
 * browser, or none), loads Frostveil's list into the Plan store, and gives
 * React Flow the browser parts jsdom lacks.
 */
import { mockIPC } from '@tauri-apps/api/mocks';
import { commands } from '@/ipc/client';
import { events } from '@/ipc/generated/bindings';
import { createBackend } from '@/ipc/mock/backend';
import { useAppStore } from '@/state/use-app-store';
import { usePlanCameraStore } from '@/state/use-plan-camera-store';
import { usePlanExtrasStore } from '@/state/use-plan-extras-store';
import { usePlanLogStore } from '@/state/use-plan-log-store';
import { usePlansStore } from '@/state/use-plans-store';

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

/**
 * `demo` gives Frostveil four canvases (`systems` adds Combat systems, a
 * Game mechanics canvas); anything else none. Plan events
 * reach the listeners, as the folder watcher's would; the backend's
 * handler comes back for the mock only commands (an agent's run, a change
 * from another computer).
 */
export async function openPlans(plans: string | null = 'demo') {
  usePlansStore.setState(usePlansStore.getInitialState(), true);
  usePlanLogStore.getState().clear();
  usePlanCameraStore.getState().clear();
  usePlanExtrasStore.getState().clear();
  usePlanLogStore.setState({ panel: false, filter: 'all', dismissed: null });
  const handle = createBackend({
    scenario: 'configured',
    probeDelay: 0,
    pace: 0,
    conceptMs: 60_000,
    plans: plans ?? undefined,
    emit: event => {
      if (event.name === 'plan')
        void events.planChanged.emit({ project: event.project, id: event.id });
      if (event.name === 'layout') void events.layoutChanged.emit({ project: event.project });
    },
  });
  mockIPC((cmd, args) => handle(cmd, (args ?? {}) as Record<string, unknown>), {
    shouldMockEvents: true,
  });
  const boot = await commands.appBootstrap();
  useAppStore.setState({ boot, workspace: boot.workspace, phase: 'app' });
  await usePlansStore.getState().loadList('frostveil');
  return {
    ...usePlansStore.getState(),
    /** Runs a mock only command, such as `mock_plan_agent`. */
    mock: (cmd: string, args: Record<string, unknown>) => handle(cmd, args),
  };
}

export { clearMocks } from './mock-ipc';
