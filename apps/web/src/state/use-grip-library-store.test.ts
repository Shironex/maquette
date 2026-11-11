import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands, type LibraryGripSummary } from '@/ipc/client';
import { SCYTHE_UID } from '@/ipc/mock/grip-library-fixtures';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import { gripNote } from './recipe-library';
import { filterGrips, useGripLibraryStore } from './use-grip-library-store';

afterEach(() => {
  clearMocks();
  useGripLibraryStore.setState(useGripLibraryStore.getInitialState(), true);
});

const store = () => useGripLibraryStore.getState();

describe('the grip library store', () => {
  it('reads the demo library and reloads on the event only once read', async () => {
    await openMockLibrary('demo');
    const list = vi.spyOn(commands, 'gripLibraryList');
    store().onChanged();
    expect(list).not.toHaveBeenCalled();
    await store().load();
    expect(store().list).toHaveLength(6);
    store().onChanged();
    expect(list).toHaveBeenCalledTimes(2);
  });

  it('keeps the newest list when an older read answers last', async () => {
    await openMockLibrary('demo');
    let release: (() => void) | undefined;
    vi.spyOn(commands, 'gripLibraryList').mockImplementationOnce(
      () =>
        new Promise<LibraryGripSummary[]>(resolve => {
          release = () => resolve([]);
        })
    );
    const first = store().load();
    await store().load();
    release?.();
    await first;
    expect(store().list).toHaveLength(6);
  });

  it('filters by weapon group', async () => {
    await openMockLibrary('demo');
    await store().load();
    const list = store().list ?? [];
    expect(filterGrips(list, 'all')).toHaveLength(6);
    expect(filterGrips(list, 'swords').map(g => g.id)).toEqual([
      'longsword-two-hands',
      'rapier-sabre-grip',
    ]);
    expect(filterGrips(list, 'polearms').map(g => g.id)).toEqual(['scythe-staff-grip']);
    expect(filterGrips(list, 'empty').map(g => g.id)).toEqual(['empty-hand-relaxed']);
  });

  it('says why the only revision cannot go, and removes a whole grip', async () => {
    await openMockLibrary('demo');
    await store().load();
    store().select('dagger-reverse');
    expect(await store().removeRevision('dagger-reverse', 1)).toMatch(/only revision/);
    expect(await store().removeRevision('longsword-two-hands', 2)).toBeNull();
    expect(store().list?.find(g => g.id === 'longsword-two-hands')?.revisions).toHaveLength(2);
    expect(await store().remove('dagger-reverse')).toBeNull();
    expect(store().selected).toBeNull();
    expect(store().list?.some(g => g.id === 'dagger-reverse')).toBe(false);
  });

  it('refuses a removal against a grip that moved since the list was read', async () => {
    await openMockLibrary('demo');
    await store().load();
    const [first] = store().list ?? [];
    useGripLibraryStore.setState({
      list: store().list?.map(g => (g.id === first?.id ? { ...g, version: 'old' } : g)) ?? null,
    });
    expect(await store().remove(first!.id)).toMatch(/changed|moved/i);
    expect(store().list?.some(g => g.id === first!.id)).toBe(true);
  });

  it('notes a newer revision, a removed grip, or nothing', async () => {
    await openMockLibrary('demo');
    await store().load();
    const list = store().list;
    const from = { entry: 'scythe-staff-grip', uid: SCYTHE_UID, revision: 1 };
    expect(gripNote(from, list)).toEqual({
      kind: 'newer',
      entry: 'scythe-staff-grip',
      name: 'Scythe, staff grip',
      latest: 2,
    });
    expect(gripNote({ ...from, revision: 2 }, list)).toBeNull();
    expect(gripNote({ ...from, uid: null }, list)).toMatchObject({ kind: 'newer' });
    expect(gripNote({ ...from, entry: 'gone' }, list)).toEqual({ kind: 'gone' });
    expect(gripNote({ ...from, uid: '0000000000000bad' }, list)).toEqual({ kind: 'gone' });
    expect(gripNote(from, null)).toBeNull();
    expect(gripNote(null, list)).toBeNull();
  });
});
