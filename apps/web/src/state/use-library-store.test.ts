import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import { NO_FILTERS } from './library';
import { useAppStore } from './use-app-store';
import { useLibraryStore } from './use-library-store';

const asset = (project: string, id: string) =>
  useAppStore
    .getState()
    .workspace?.projects.find(p => p.id === project)
    ?.assets?.find(a => a.id === id);

beforeEach(async () => {
  await openMockLibrary('demo');
});

afterEach(() => {
  clearMocks();
  useLibraryStore.setState({
    view: null,
    filters: NO_FILTERS,
    selected: null,
    adding: null,
    updating: null,
    plan: null,
    planError: null,
  });
});

describe('useLibraryStore', () => {
  it('loads the entries and keeps a selection that still exists', async () => {
    useLibraryStore.setState({ selected: 'character' });
    await useLibraryStore.getState().load();
    expect(useLibraryStore.getState().view?.items).toHaveLength(8);
    expect(useLibraryStore.getState().selected).toBe('character');
  });

  it('uses a model in a project and says it spent nothing', async () => {
    await useLibraryStore.getState().load();
    await useLibraryStore.getState().use('stone-lantern', null, 'rooftop-cinema');
    expect(asset('rooftop-cinema', 'stone-lantern')?.origin).toEqual({
      entry: 'stone-lantern',
      version: 1,
    });
    expect(useAppStore.getState().notice?.title).toBe(
      'Added Stone lantern to Rooftop cinema, no credits'
    );
  });

  it('says why a model cannot be used', async () => {
    await useLibraryStore.getState().use('crypt-brazier', null, 'moon-island');
    expect(useAppStore.getState().notice?.status).toBe('failed');
  });

  it('reads the plan before an update and updates only on confirm', async () => {
    await useLibraryStore.getState().load();
    await useLibraryStore.getState().openUpdate({ project: 'moon-island', asset: 'character' });
    const plan = useLibraryStore.getState().plan;
    expect([plan?.from, plan?.to]).toEqual([2, 3]);
    expect(asset('moon-island', 'character')?.origin?.version).toBe(2);
    await useLibraryStore.getState().update();
    expect(asset('moon-island', 'character')?.origin?.version).toBe(3);
    expect(useLibraryStore.getState().updating).toBeNull();
    expect(useAppStore.getState().notice?.detail).toMatch(/one setting no longer applied/);
  });

  it('adds a newer model as the next version of its entry and selects it', async () => {
    useLibraryStore.getState().openAdd({ project: 'moon-island', asset: 'shop-house' });
    await useLibraryStore
      .getState()
      .add({ project: 'moon-island', asset: 'shop-house', name: null, tags: null });
    expect(useLibraryStore.getState().adding).toBeNull();
    expect(useLibraryStore.getState().selected).toBe('shop-house');
    expect(asset('moon-island', 'shop-house')?.origin).toEqual({ entry: 'shop-house', version: 2 });
    expect(useAppStore.getState().notice?.title).toBe('Added Shop house v2 to the library');
  });
});
