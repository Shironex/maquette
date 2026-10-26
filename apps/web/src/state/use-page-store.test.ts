import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useAppStore } from './use-app-store';
import { NO_READING, usePageStore } from './use-page-store';

const INITIAL = usePageStore.getState();

async function open(page?: string) {
  await openMockWorkspace(
    'configured',
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    page
  );
  await usePageStore.getState().open('moon-island');
}

/** Lets the mock server answer, then reads the view again as an event would. */
async function settle() {
  await new Promise(resolve => setTimeout(resolve, 5));
  await usePageStore.getState().refresh();
}

afterEach(() => {
  vi.useRealTimers();
  clearMocks();
  usePageStore.setState(INITIAL, true);
});

describe('usePageStore', () => {
  it('starts the site dev server when the view opens', async () => {
    await open();
    expect(usePageStore.getState().view?.server.state).toBe('starting');
    expect(usePageStore.getState().view?.command?.source).toBe('package-json');
    await settle();
    const server = usePageStore.getState().view?.server;
    expect(server?.state).toBe('running');
    expect(server?.url).toBe('http://localhost:4321/');
    expect(server?.owner).toBe('page');
  });

  it('loads the inspect numbers with the last ship on the site', async () => {
    await open('running');
    await vi.waitFor(() => expect(usePageStore.getState().inspect).not.toBeNull());
    const sources = new Set(usePageStore.getState().inspect?.models.map(m => m.source));
    expect(sources).toEqual(new Set(['shipped', 'site']));
  });

  it('lets go of the server it started when the view closes', async () => {
    await open();
    await settle();
    vi.useFakeTimers();
    usePageStore.getState().close('moon-island');
    await vi.advanceTimersByTimeAsync(500);
    vi.useRealTimers();
    expect((await commands.pageGet('moon-island')).server.state).toBe('idle');
  });

  it('keeps the server when the view comes back quickly', async () => {
    await open();
    await settle();
    vi.useFakeTimers();
    usePageStore.getState().close('moon-island');
    await vi.advanceTimersByTimeAsync(100);
    await usePageStore.getState().open('moon-island');
    await vi.advanceTimersByTimeAsync(1000);
    vi.useRealTimers();
    expect((await commands.pageGet('moon-island')).server.state).toBe('running');
  });

  it('says why a failed server stopped', async () => {
    await open('failed');
    await settle();
    const server = usePageStore.getState().view?.server;
    expect(server?.state).toBe('failed');
    expect(server?.message).toContain('stopped before it answered');
  });

  it('starts nothing without a site repo', async () => {
    await open('no-site');
    const view = usePageStore.getState().view;
    expect(view?.siteRepo).toBeNull();
    expect(view?.server.state).toBe('idle');
    expect(view?.commandError).toContain('Settings > Project');
  });

  it('saves the dev command and switches to a page concept', async () => {
    await open();
    expect(await usePageStore.getState().save({ devCommand: 'pnpm astro dev --port {port}' })).toBe(
      true
    );
    expect(usePageStore.getState().view?.command?.source).toBe('settings');
    await usePageStore.getState().pickConcept();
    const settings = usePageStore.getState().view?.settings;
    expect(settings?.source).toBe('concept');
    expect(settings?.concept).toMatch(/concepts\/home_v2\.html$/);
    expect(await usePageStore.getState().save({ path: 'pricing' })).toBe(false);
    expect(useAppStore.getState().notice?.title).toBe('Could not save the page settings');
  });

  it('keeps what each frame reports', async () => {
    await open();
    const { onMessage, forgetFrame } = usePageStore.getState();
    onMessage('main', { type: 'ready', stops: [{ name: 'Porch' }], features: [] });
    onMessage('main', { type: 'at', stop: 0.5, scroll: { top: 400, max: 3200 } });
    onMessage('main', { type: 'stats', frameMs: 9.4, p95Ms: 12.1, drawCalls: 86 });
    expect(usePageStore.getState().readings.main).toEqual({
      stops: [{ name: 'Porch' }],
      at: 0.5,
      scroll: { top: 400, max: 3200 },
      stats: { frameMs: 9.4, p95Ms: 12.1, drawCalls: 86 },
    });
    forgetFrame('main');
    expect(usePageStore.getState().readings.main ?? NO_READING).toEqual(NO_READING);
  });

  it('follows server events for this project and another', async () => {
    await open();
    await settle();
    const view = usePageStore.getState().view!;
    usePageStore.getState().onServer({ ...view.server, project: 'lantern-keeper' });
    expect(usePageStore.getState().view?.busyWith).toBe('lantern-keeper');
    expect(usePageStore.getState().view?.server.state).toBe('idle');
    usePageStore.getState().onServer({ ...view.server, state: 'idle', project: null });
    expect(usePageStore.getState().view?.busyWith).toBeNull();
  });

  it('counts each seek so a frame sends it again', () => {
    usePageStore.getState().seek(2);
    usePageStore.getState().seek(2, true);
    const s = usePageStore.getState();
    expect([s.target, s.seekId, s.smooth]).toEqual([2, 2, true]);
  });
});
