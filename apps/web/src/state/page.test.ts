import { describe, expect, it } from 'vitest';
import type { PageView, ServerStatus } from '@/ipc/client';
import {
  device,
  fitLabel,
  fitScale,
  frameSource,
  frameState,
  goto,
  pageAddress,
  readPageMessage,
  scrollLine,
  serverLine,
  shortAddress,
  shouldAutoStart,
  siteName,
  stepStop,
} from './page';

const IDLE: ServerStatus = {
  state: 'idle',
  project: null,
  site: null,
  url: null,
  command: null,
  owner: null,
  external: false,
  startedAt: null,
  message: null,
  log: [],
};

function view(patch: Partial<PageView> = {}, server: Partial<ServerStatus> = {}): PageView {
  return {
    project: 'moon-island',
    siteRepo: '/Users/maker/Projects/moon-island-site',
    settings: { version: 1, devCommand: null, concept: null, source: 'site', path: '/' },
    command: null,
    commandError: null,
    siteConfig: null,
    busyWith: null,
    ...patch,
    server: { ...IDLE, ...server },
  };
}

describe('page devices', () => {
  it('fits the device viewport into the box, never above full size', () => {
    const desktop = device('desktop');
    expect(fitScale(desktop, { width: 835, height: 600 })).toBeCloseTo(0.58, 2);
    expect(fitScale(device('phone'), { width: 2000, height: 2000 })).toBe(1);
    expect(fitScale(desktop, { width: 0, height: 500 })).toBe(0);
    expect(fitLabel(desktop, 0.58)).toBe('Desktop 1440 × 900 · shown at 58%');
    expect(fitLabel(device('phone'), 1)).toBe('Phone 390 × 844 · shown at full size');
  });
});

describe('page addresses', () => {
  it('joins the path and maps [::1] to localhost', () => {
    expect(pageAddress('http://localhost:4321/', '/pricing')).toBe('http://localhost:4321/pricing');
    expect(pageAddress('http://[::1]:5173', '/')).toBe('http://localhost:5173/');
    expect(pageAddress('http://127.0.0.1:5173/', '/', { poster: true })).toBe(
      'http://127.0.0.1:5173/?maquette-poster=1'
    );
    expect(pageAddress(null, '/')).toBeNull();
    expect(pageAddress('not a url', '/')).toBeNull();
  });

  it('keeps a concept file as it is', () => {
    expect(pageAddress('asset://localhost/%2Fws%2Fhome.html', '/pricing')).toBe(
      'asset://localhost/%2Fws%2Fhome.html'
    );
  });

  it('shows the address without its scheme', () => {
    expect(shortAddress('http://localhost:4321/')).toBe('localhost:4321/');
    expect(shortAddress('asset://localhost/%2Fws%2Fconcepts%2Fhome_v2.html')).toBe(
      '/ws/concepts/home_v2.html'
    );
    expect(siteName('/Users/maker/Projects/moon-island-site/')).toBe('moon-island-site');
    expect(siteName('C:\\Sites\\moon')).toBe('moon');
    expect(siteName(null)).toBe('No site');
  });
});

describe('the frame source', () => {
  it('loads the running dev server at the path, and names the server until then', () => {
    expect(frameSource(view())).toEqual({ src: null, address: 'moon-island-site dev server' });
    const running = view(
      { settings: { version: 1, devCommand: null, concept: null, source: 'site', path: '/a' } },
      { state: 'running', url: 'http://localhost:5173/' }
    );
    expect(frameSource(running)).toEqual({
      src: 'http://localhost:5173/a',
      address: 'localhost:5173/a',
    });
    expect(frameSource(running, { poster: true })).toEqual({
      src: 'http://localhost:5173/a?maquette-poster=1',
      address: 'localhost:5173/a',
    });
  });

  it('names a page concept by its file', () => {
    const concept = view({
      settings: {
        version: 1,
        devCommand: null,
        concept: '/ws/concepts/home_v2.html',
        source: 'concept',
        path: '/',
      },
    });
    expect(frameSource(concept).address).toBe('home_v2.html');
    expect(frameSource(null)).toEqual({ src: null, address: '' });
  });
});

describe('the page bridge', () => {
  it('reads ready, at and stats from the page', () => {
    expect(
      readPageMessage({
        maquette: 1,
        type: 'ready',
        stops: [{ name: 'Hero' }, { name: '' }, 4, { name: 'Ship' }],
        features: ['goto', 3],
      })
    ).toEqual({ type: 'ready', stops: [{ name: 'Hero' }, { name: 'Ship' }], features: ['goto'] });
    expect(
      readPageMessage({ maquette: 1, type: 'at', stop: 1.25, scroll: { top: 900, max: 5000 } })
    ).toEqual({ type: 'at', stop: 1.25, scroll: { top: 900, max: 5000 } });
    expect(readPageMessage({ maquette: 1, type: 'at', stop: 2, scroll: 'x' })).toEqual({
      type: 'at',
      stop: 2,
      scroll: null,
    });
    expect(readPageMessage({ maquette: 1, type: 'stats', frameMs: 9.4, p95Ms: 12.1 })).toEqual({
      type: 'stats',
      frameMs: 9.4,
      p95Ms: 12.1,
      drawCalls: null,
    });
  });

  it('ignores other messages', () => {
    for (const data of [null, 'ready', { type: 'ready', stops: [] }, { maquette: 1, type: 'at' }]) {
      expect(readPageMessage(data)).toBeNull();
    }
  });

  it('sends goto rounded', () => {
    expect(goto(1.23456)).toEqual({ maquette: 1, type: 'goto', stop: 1.235, smooth: false });
  });
});

describe('the scrubber', () => {
  const stops = [{ name: 'Hero' }, { name: 'Services' }, { name: 'Switch room' }];

  it('says where the page is', () => {
    expect(scrollLine(1, stops, { top: 1850, max: 5960 })).toEqual({
      percent: '31%',
      detail: '1,850 of 5,960 px · stop 2, Services',
    });
    expect(scrollLine(2, stops, null)).toEqual({ percent: '100%', detail: 'stop 3, Switch room' });
  });

  it('steps to the next and previous stop', () => {
    expect(stepStop(0, 1, 3)).toBe(1);
    expect(stepStop(0.4, 1, 3)).toBe(1);
    expect(stepStop(1, 1, 3)).toBe(2);
    expect(stepStop(2, 1, 3)).toBe(2);
    expect(stepStop(1.6, -1, 3)).toBe(1);
    expect(stepStop(1, -1, 3)).toBe(0);
    expect(stepStop(0, -1, 3)).toBe(0);
  });
});

describe('the frame state', () => {
  it('follows the server and the source', () => {
    expect(frameState(view({ siteRepo: null }), null).kind).toBe('no-site');
    expect(frameState(view({}, { state: 'starting' }), null).kind).toBe('starting');
    expect(frameState(view({}, { state: 'failed', message: 'boom' }), null).kind).toBe('failed');
    expect(frameState(view({}, { state: 'running', url: 'http://localhost:4321/' }), null)).toEqual(
      expect.objectContaining({ kind: 'running', url: 'http://localhost:4321/' })
    );
    expect(frameState(view({ commandError: 'No dev script' }), null)).toEqual({
      kind: 'no-command',
      message: 'No dev script',
    });
    expect(frameState(view({ busyWith: 'lantern-keeper' }), null)).toEqual({
      kind: 'busy',
      project: 'lantern-keeper',
    });
    const concept = view({
      settings: {
        version: 1,
        devCommand: null,
        concept: '/ws/c.html',
        source: 'concept',
        path: '/',
      },
    });
    expect(frameState(concept, 'asset://c').kind).toBe('running');
    expect(frameState(concept, null).kind).toBe('no-concept');
  });

  it('starts the server on open only when nothing else uses it', () => {
    expect(shouldAutoStart(view())).toBe(true);
    expect(shouldAutoStart(view({ siteRepo: null }))).toBe(false);
    expect(shouldAutoStart(view({ busyWith: 'lantern-keeper' }))).toBe(false);
    expect(shouldAutoStart(view({ commandError: 'none' }))).toBe(false);
    expect(shouldAutoStart(view({}, { state: 'failed' }))).toBe(false);
  });

  it('says how the server stands', () => {
    expect(serverLine({ ...IDLE, state: 'running', url: 'http://localhost:4321/' })).toBe(
      'localhost:4321 · running'
    );
    expect(serverLine({ ...IDLE, state: 'running', url: 'http://x:1/', external: true })).toBe(
      'x:1 · already running'
    );
    expect(serverLine({ ...IDLE, state: 'starting' })).toBe('starting');
    expect(serverLine(IDLE)).toBe('stopped');
  });
});
