import { afterEach, describe, expect, it } from 'vitest';
import { useAnimationsStore } from './use-animations-store';
import { useRecipesStore } from './use-recipes-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';

afterEach(() => {
  clearMocks();
  useRecipesStore.setState({ project: null, list: null, error: null, picking: null });
  useAnimationsStore.setState({ view: null });
});

describe('the recipes store', () => {
  it('makes a recipe from a library clip and lists it', async () => {
    await openMockLibrary();
    await useRecipesStore.getState().load('frostveil');
    expect(useRecipesStore.getState().list).toEqual([]);
    useRecipesStore.getState().openPicker('frostveil');
    const refused = await useRecipesStore.getState().create('no-such-clip', '');
    expect(refused).toBeNull();
    expect(useRecipesStore.getState().pickError).toContain('no-such-clip');
    expect(useRecipesStore.getState().picking).toBe('frostveil');
  });

  it('lists the demo recipes and makes one named after its clip', async () => {
    await openMockLibrary('demo');
    await useAnimationsStore.getState().load();
    const clip = useAnimationsStore.getState().view!.items[0]!.clip;
    await useRecipesStore.getState().load('frostveil');
    const before = useRecipesStore.getState().list!.length;
    expect(before).toBeGreaterThan(0);
    useRecipesStore.getState().openPicker('frostveil');
    const doc = await useRecipesStore.getState().create(clip.id, '  ');
    expect(doc?.recipe.name).toBe(clip.name);
    expect(useRecipesStore.getState().picking).toBeNull();
    expect(useRecipesStore.getState().list).toHaveLength(before + 1);
  });
});
