import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { bakeKey } from './recipe-bake';
import { record } from '@/test/recipe-bake-fixture';
import { useAppStore } from './use-app-store';
import { useRecipeBakesStore } from './use-recipe-bakes-store';

const KEY = bakeKey('frostveil', 'reach-out');

afterEach(() => {
  vi.restoreAllMocks();
  useRecipeBakesStore.setState(useRecipeBakesStore.getInitialState(), true);
});

describe('the recipe bakes store', () => {
  it('starts a bake with the version read and keeps its record', async () => {
    const running = record({ state: 'running', result: null });
    const start = vi.spyOn(commands, 'recipeBakeStart').mockResolvedValue(running);
    const ok = await useRecipeBakesStore
      .getState()
      .start('frostveil', 'reach-out', 'elsheria', 'v1');
    expect(ok).toBe(true);
    expect(start).toHaveBeenCalledWith({
      project: 'frostveil',
      recipe: 'reach-out',
      asset: 'elsheria',
      expected: 'v1',
    });
    expect(useRecipeBakesStore.getState().bakes[KEY]?.state).toBe('running');
    expect(useRecipeBakesStore.getState().busy[KEY]).toBeUndefined();
  });

  it('keeps what a refused start said', async () => {
    vi.spyOn(commands, 'recipeBakeStart').mockRejectedValue({
      code: 'invalid',
      message: 'Fill the open blanks of Reach out first: cup.',
    });
    const ok = await useRecipeBakesStore.getState().start('frostveil', 'reach-out', 'elsheria');
    expect(ok).toBe(false);
    expect(useRecipeBakesStore.getState().errors[KEY]).toMatch(/open blanks/);
  });

  it('takes the newer record when an event overtakes an answer', () => {
    const store = useRecipeBakesStore.getState();
    store.onChanged(record({ state: 'done', updatedMs: 50 }));
    store.onChanged(record({ state: 'running', updatedMs: 10, result: null }));
    expect(useRecipeBakesStore.getState().bakes[KEY]?.state).toBe('done');
  });

  it('reads a recipe never baked as none, and asks the core to cancel', async () => {
    vi.spyOn(commands, 'recipeBakeGet').mockResolvedValue(null);
    await useRecipeBakesStore.getState().load('frostveil', 'reach-out');
    expect(useRecipeBakesStore.getState().bakes[KEY]).toBeNull();
    const cancel = vi.spyOn(commands, 'recipeBakeCancel').mockResolvedValue(null);
    vi.spyOn(commands, 'recipeBakeGet').mockResolvedValue(record({ state: 'cancelled' }));
    await useRecipeBakesStore.getState().cancel('frostveil', 'reach-out');
    expect(cancel).toHaveBeenCalledWith('frostveil', 'reach-out');
    expect(useRecipeBakesStore.getState().bakes[KEY]?.state).toBe('cancelled');
  });

  it('adds the reviewed bake, closes the review and says what happened', async () => {
    vi.spyOn(commands, 'recipeBakeGet').mockResolvedValue(record());
    const store = useRecipeBakesStore.getState();
    store.openReview('frostveil', 'reach-out');
    expect(useRecipeBakesStore.getState().reviewing).toEqual({
      project: 'frostveil',
      recipe: 'reach-out',
    });
    const added = record({
      state: 'added',
      addedVersion: 3,
      updatedMs: 9,
      result: {
        ...record().result!,
        clipNote: 'Replaces reach_out, the clip an earlier bake added.',
      },
    });
    const add = vi.spyOn(commands, 'recipeBakeAdd').mockResolvedValue(added);
    expect(await store.add('frostveil', 'reach-out', 5)).toBe(true);
    expect(add).toHaveBeenCalledWith('frostveil', 'reach-out', 5);
    const after = useRecipeBakesStore.getState();
    expect(after.reviewing).toBeNull();
    expect(after.bakes[KEY]?.state).toBe('added');
    expect(useAppStore.getState().notice).toMatchObject({
      title: 'Added as v3 of Elsheria.',
      detail: 'Replaces reach_out, the clip an earlier bake added.',
    });
  });

  it('keeps the review open with why an add was refused', async () => {
    vi.spyOn(commands, 'recipeBakeGet').mockResolvedValue(record());
    vi.spyOn(commands, 'recipeBakeAdd').mockRejectedValue({
      code: 'invalid',
      message: 'Not added: this bake is out of date. The recipe changed since this bake.',
    });
    const store = useRecipeBakesStore.getState();
    store.openReview('frostveil', 'reach-out');
    expect(await store.add('frostveil', 'reach-out', 5)).toBe(false);
    expect(useRecipeBakesStore.getState().reviewing).not.toBeNull();
    expect(useRecipeBakesStore.getState().addErrors[KEY]).toMatch(/out of date/);
  });
});
