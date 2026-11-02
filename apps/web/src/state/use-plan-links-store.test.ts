import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands, type PlanLinks } from '@/ipc/client';
import { clearMocks, openPlans } from '@/test/open-plan';
import { usePlanLinksStore } from './use-plan-links-store';

afterEach(() => {
  vi.restoreAllMocks();
  usePlanLinksStore.getState().clear();
  clearMocks();
});

const gone = (id: string): PlanLinks => ({
  assets: [
    {
      id,
      state: 'gone',
      name: null,
      kind: null,
      stage: null,
      thumbnail: null,
      version: null,
      file: null,
      stats: null,
      checks: 0,
    },
  ],
  directions: [],
});

describe('the plan links store', () => {
  it('reads the linked assets live from the project', async () => {
    await openPlans();
    await usePlanLinksStore
      .getState()
      .refresh('frostveil', { assets: ['elsheria', 'frost-warden'], directions: [] });
    const s = usePlanLinksStore.getState();
    expect(s.assets.elsheria?.state).toBe('ready');
    expect(s.assets.elsheria?.name).toBe('Elsheria');
    expect(s.assets['frost-warden']?.state).toBe('gone');
  });

  it('drops a read that answers after a newer one', async () => {
    let answer: (links: PlanLinks) => void = () => {};
    vi.spyOn(commands, 'planLinks')
      .mockImplementationOnce(() => new Promise(resolve => (answer = resolve)))
      .mockResolvedValueOnce(gone('new'));
    const first = usePlanLinksStore
      .getState()
      .refresh('frostveil', { assets: ['old'], directions: [] });
    await usePlanLinksStore.getState().refresh('frostveil', { assets: ['new'], directions: [] });
    answer(gone('old'));
    await first;
    expect(Object.keys(usePlanLinksStore.getState().assets)).toEqual(['new']);
  });

  it('keeps the last read when a read fails, and asks nothing for no links', async () => {
    const read = vi.spyOn(commands, 'planLinks').mockResolvedValueOnce(gone('a'));
    await usePlanLinksStore.getState().refresh('frostveil', { assets: ['a'], directions: [] });
    read.mockRejectedValueOnce({ code: 'io', message: 'Locked' });
    await usePlanLinksStore.getState().refresh('frostveil', { assets: ['a'], directions: [] });
    expect(usePlanLinksStore.getState().error).toBe('Locked');
    expect(usePlanLinksStore.getState().assets.a).toBeDefined();

    await usePlanLinksStore.getState().refresh('frostveil', { assets: [], directions: [] });
    expect(read).toHaveBeenCalledTimes(2);
    expect(usePlanLinksStore.getState().assets).toEqual({});
  });

  it('starts empty for another project', async () => {
    vi.spyOn(commands, 'planLinks')
      .mockResolvedValueOnce(gone('a'))
      .mockImplementationOnce(() => new Promise(() => {}));
    await usePlanLinksStore.getState().refresh('frostveil', { assets: ['a'], directions: [] });
    void usePlanLinksStore.getState().refresh('moon-island', { assets: ['a'], directions: [] });
    expect(usePlanLinksStore.getState()).toMatchObject({ project: 'moon-island', assets: {} });
  });
});
