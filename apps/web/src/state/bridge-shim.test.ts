/**
 * The page bridge shim the frame proxy adds to a framed page
 * (crates/maquette-core/src/site/preview/frame/shim.js), run against a fake
 * window: its stop mapping, what it sends, and how it stands down for a
 * page with its own bridge.
 */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { readPageMessage } from './page';

const SHIM = readFileSync(
  resolve(import.meta.dirname, '../../../../crates/maquette-core/src/site/preview/frame/shim.js'),
  'utf8'
);

const VIEW = 800;
const STOPS = ['Porch', 'Sakura', 'Shrubs', 'Canopy', 'Pullback'].map(name => ({ name }));

type Listener = (event: unknown) => void;

interface FakeWindow {
  parent: { postMessage: ReturnType<typeof vi.fn> };
  scrollY: number;
  __maquetteBridge?: number;
  scrollTo: ReturnType<typeof vi.fn>;
  fire: (type: string, event?: unknown) => void;
  /** Moves the clock on by `ms` and runs the frames due. */
  tick: (ms: number) => void;
  hello: (stops?: unknown) => void;
  sent: () => Array<Record<string, unknown>>;
}

/** A page of `height` px and the shim running in it, as if framed. */
function framed(height: number): FakeWindow {
  const listeners = new Map<string, Set<Listener>>();
  const frames = new Map<number, (now: number) => void>();
  let nextFrame = 0;
  let clock = 0;
  Object.defineProperty(document.documentElement, 'scrollHeight', {
    configurable: true,
    value: height,
  });
  const win = {
    document,
    innerHeight: VIEW,
    scrollY: 0,
    parent: { postMessage: vi.fn() },
    performance: { now: () => clock },
    getComputedStyle: () => ({ scrollPaddingTop: '0px', scrollMarginTop: '0px' }),
    setTimeout: (fn: () => void, ms: number) => setTimeout(fn, ms),
    requestAnimationFrame: (fn: (now: number) => void) => {
      nextFrame += 1;
      frames.set(nextFrame, fn);
      return nextFrame;
    },
    cancelAnimationFrame: (id: number) => frames.delete(id),
    addEventListener: (type: string, fn: Listener) => {
      if (!listeners.has(type)) listeners.set(type, new Set());
      listeners.get(type)!.add(fn);
    },
    removeEventListener: (type: string, fn: Listener) => listeners.get(type)?.delete(fn),
  } as unknown as FakeWindow & Record<string, unknown>;
  win.scrollTo = vi.fn(({ top }: { top: number }) => {
    win.scrollY = top;
  });
  win.fire = (type, event = {}) => {
    for (const fn of [...(listeners.get(type) ?? [])]) fn(event);
    win.tick(0);
  };
  win.tick = ms => {
    clock += ms;
    const due = [...frames.values()];
    frames.clear();
    for (const fn of due) fn(clock);
  };
  win.hello = (stops = STOPS) =>
    win.fire('message', {
      source: win.parent,
      origin: 'http://localhost:15303',
      data: { maquette: 1, type: 'hello', stops },
    });
  win.sent = () => win.parent.postMessage.mock.calls.map(c => c[0] as Record<string, unknown>);
  new Function('window', SHIM)(win);
  return win;
}

function goto(win: FakeWindow, stop: number, smooth = false) {
  win.fire('message', {
    source: win.parent,
    origin: 'http://localhost:15303',
    data: { maquette: 1, type: 'goto', stop, smooth },
  });
}

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  document.body.innerHTML = '';
});

describe('the page bridge shim', () => {
  it('answers hello once the page has loaded, as the shim, with the stops it was given', () => {
    const win = framed(4000);
    win.hello();
    expect(win.sent()).toEqual([]);
    vi.advanceTimersByTime(300);
    const ready = win.sent()[0];
    expect(ready).toMatchObject({ maquette: 1, type: 'ready', source: 'shim', stops: STOPS });
    expect(readPageMessage(ready)).toMatchObject({ type: 'ready', source: 'shim' });
    expect(win.parent.postMessage).toHaveBeenCalledWith(ready, 'http://localhost:15303');
  });

  it('spreads the stops evenly over the scroll range without anchors', () => {
    const win = framed(4000);
    win.hello();
    vi.advanceTimersByTime(300);
    goto(win, 2);
    expect(win.scrollTo).toHaveBeenLastCalledWith({ top: 1600, behavior: 'instant' });
    goto(win, 3.5, true);
    expect(win.scrollTo).toHaveBeenLastCalledWith({ top: 2800, behavior: 'smooth' });
    goto(win, 9);
    expect(win.scrollTo).toHaveBeenLastCalledWith({ top: 3200, behavior: 'instant' });
  });

  it('scrolls to the element that names a stop, by id or data-maquette-stop', () => {
    document.body.innerHTML =
      '<section id="sakura"></section><section data-maquette-stop="Canopy"></section>';
    const win = framed(4000);
    const [sakura, canopy] = [...document.querySelectorAll('section')];
    sakura!.getBoundingClientRect = () => ({ top: 500 - win.scrollY }) as DOMRect;
    canopy!.getBoundingClientRect = () => ({ top: 2000 - win.scrollY }) as DOMRect;
    win.hello();
    vi.advanceTimersByTime(300);
    goto(win, 1);
    expect(win.scrollTo).toHaveBeenLastCalledWith({ top: 500, behavior: 'instant' });
    goto(win, 3);
    expect(win.scrollTo).toHaveBeenLastCalledWith({ top: 2000, behavior: 'instant' });
    // Stop 3 has no element: it sits where an even spread puts it.
    goto(win, 2);
    expect(win.scrollTo).toHaveBeenLastCalledWith({ top: 1600, behavior: 'instant' });
  });

  it('says where a scroll left the page', () => {
    const win = framed(4000);
    win.hello();
    vi.advanceTimersByTime(300);
    win.parent.postMessage.mockClear();
    win.scrollY = 2000;
    win.fire('scroll');
    expect(win.sent()).toContainEqual({
      maquette: 1,
      source: 'shim',
      type: 'at',
      stop: 2.5,
      scroll: { top: 2000, max: 3200 },
    });
  });

  it('sends frame times about once a second, with no draw calls', () => {
    const win = framed(4000);
    win.hello();
    vi.advanceTimersByTime(300);
    const stats = () => win.sent().filter(m => m.type === 'stats');
    for (let i = 0; i < 70; i++) win.tick(16);
    expect(stats()).toEqual([
      { maquette: 1, source: 'shim', type: 'stats', frameMs: 16, p95Ms: 16 },
    ]);
    for (let i = 0; i < 60; i++) win.tick(i % 5 === 0 ? 50 : 16);
    expect(stats()).toHaveLength(2);
    expect(stats()[1]!.p95Ms).toBe(50);
  });

  it('stays silent for a page with its own bridge', () => {
    const win = framed(4000);
    win.__maquetteBridge = 1;
    win.hello();
    vi.advanceTimersByTime(300);
    goto(win, 2);
    expect(win.sent()).toEqual([]);
    expect(win.scrollTo).not.toHaveBeenCalled();
  });

  it('stands down when the page brings up its own bridge later', () => {
    const win = framed(4000);
    win.hello();
    vi.advanceTimersByTime(300);
    win.__maquetteBridge = 1;
    win.parent.postMessage.mockClear();
    win.hello();
    goto(win, 2);
    win.scrollY = 900;
    win.fire('scroll');
    expect(win.sent()).toEqual([]);
    expect(win.scrollTo).not.toHaveBeenCalled();
  });

  it('hears only its parent window', () => {
    const win = framed(4000);
    win.fire('message', {
      source: {},
      origin: 'http://evil.test',
      data: { maquette: 1, type: 'hello' },
    });
    vi.advanceTimersByTime(300);
    expect(win.sent()).toEqual([]);
  });
});
