import { afterEach, describe, expect, it, vi } from 'vitest';
import { BRIDGE_VERSION, frameStats, readMessage, startBridge } from './bridge';

describe('readMessage', () => {
  it('reads hello and goto from Maquette', () => {
    expect(readMessage({ maquette: 1, type: 'hello' })).toEqual({ maquette: 1, type: 'hello' });
    expect(readMessage({ maquette: 1, type: 'goto', stop: 2.5 })).toEqual({
      maquette: 1,
      type: 'goto',
      stop: 2.5,
      smooth: false,
    });
    expect(readMessage({ maquette: 1, type: 'goto', stop: 1, smooth: true })?.type).toBe('goto');
  });

  it('ignores anything else on the channel', () => {
    for (const data of [
      null,
      'hello',
      { type: 'hello' },
      { maquette: 2, type: 'hello' },
      { maquette: 1, type: 'goto' },
      { maquette: 1, type: 'goto', stop: Number.NaN },
      { maquette: 1, type: 'reload' },
    ]) {
      expect(readMessage(data)).toBeNull();
    }
  });
});

describe('frameStats', () => {
  it('gives the mean and the 95th percentile of frame intervals', () => {
    const deltas = [
      ...Array.from({ length: 95 }, () => 16),
      ...Array.from({ length: 5 }, () => 40),
    ];
    expect(frameStats(deltas)).toEqual({ frameMs: 17.2, p95Ms: 40 });
    expect(frameStats([10])).toEqual({ frameMs: 10, p95Ms: 10 });
  });

  it('has nothing to say before the first frame', () => {
    expect(frameStats([])).toBeNull();
  });
});

describe('startBridge', () => {
  const options = { stops: [], read: () => 0, subscribe: () => () => {}, goto: () => {} };

  /** A window in a frame, or (`framed` false) one on its own. */
  function stubWindow(framed: boolean): Partial<Window> {
    const win: Partial<Window> = { addEventListener: vi.fn() };
    Object.assign(win, { parent: framed ? { postMessage: vi.fn() } : win });
    vi.stubGlobal('window', win);
    return win;
  }

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("tells Maquette's shim to stand down when framed", () => {
    const win = stubWindow(true);
    startBridge(options);
    expect(win.__maquetteBridge).toBe(BRIDGE_VERSION);
  });

  it('stays off outside a frame', () => {
    const win = stubWindow(false);
    startBridge(options);
    expect(win.__maquetteBridge).toBeUndefined();
    expect(win.addEventListener).not.toHaveBeenCalled();
  });
});
