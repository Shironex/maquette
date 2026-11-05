import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { bakeKey } from './recipe-bake';
import { record } from '@/test/recipe-bake-fixture';
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
});
