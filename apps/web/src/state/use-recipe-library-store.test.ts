import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands, type LibraryRecipeSummary } from '@/ipc/client';
import { REACH_UID } from '@/ipc/mock/recipe-library-fixtures';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import { libraryNote } from './recipe-library';
import { useRecipeLibraryStore } from './use-recipe-library-store';

afterEach(() => {
  clearMocks();
  useRecipeLibraryStore.setState(useRecipeLibraryStore.getInitialState(), true);
});

const store = () => useRecipeLibraryStore.getState();

describe('the recipe library store', () => {
  it('reads the demo library and reloads on the event only once read', async () => {
    await openMockLibrary('demo');
    const list = vi.spyOn(commands, 'recipeLibraryList');
    store().onChanged();
    expect(list).not.toHaveBeenCalled();
    await store().load();
    expect(store().list?.map(e => e.id)).toEqual([
      'guard-and-look',
      'reach-for-the-lantern',
      'wave-hello',
    ]);
    store().onChanged();
    expect(list).toHaveBeenCalledTimes(2);
  });

  it('keeps the newest list when an older read answers last', async () => {
    await openMockLibrary('demo');
    let release: (() => void) | undefined;
    vi.spyOn(commands, 'recipeLibraryList').mockImplementationOnce(
      () =>
        new Promise<LibraryRecipeSummary[]>(resolve => {
          release = () => resolve([]);
        })
    );
    const first = store().load();
    await store().load();
    release?.();
    await first;
    expect(store().list).toHaveLength(3);
  });

  it('imports a file, selects it and keeps what to know', async () => {
    await openMockLibrary('demo');
    await store().importFile();
    expect(store().selected).toBe('sit-and-wave');
    expect(store().imported?.notes.join(' ')).toMatch(/hash differs/);
    expect(store().list?.some(e => e.id === 'sit-and-wave')).toBe(true);
  });

  it('says why the only revision cannot go, and removes a whole recipe', async () => {
    await openMockLibrary('demo');
    await store().load();
    store().select('wave-hello');
    expect(await store().removeRevision('wave-hello', 1)).toMatch(/only revision of Wave hello/);
    expect(await store().removeRevision('reach-for-the-lantern', 2)).toBeNull();
    expect(await store().remove('wave-hello')).toBeNull();
    expect(store().selected).toBeNull();
    expect(store().list?.map(e => e.id)).toEqual(['guard-and-look', 'reach-for-the-lantern']);
  });

  it('notes a newer revision, a removed entry, or nothing', async () => {
    await openMockLibrary('demo');
    await store().load();
    const list = store().list;
    const from = { entry: 'reach-for-the-lantern', uid: REACH_UID, revision: 1 };
    expect(libraryNote(from, list)).toEqual({
      kind: 'newer',
      entry: 'reach-for-the-lantern',
      latest: 3,
    });
    expect(libraryNote({ ...from, revision: 3 }, list)).toBeNull();
    expect(libraryNote({ ...from, entry: 'gone' }, list)).toEqual({ kind: 'gone' });
    expect(libraryNote({ ...from, uid: '0000000000000bad' }, list)).toEqual({ kind: 'gone' });
    expect(libraryNote(from, null)).toBeNull();
  });
});
