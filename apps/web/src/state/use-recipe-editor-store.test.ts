import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, type RecipeLayer } from '@/ipc/client';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import { newLayer } from './recipe-draft';
import {
  ANSWER_FIRST,
  CHANGED_OUTSIDE,
  FIX_FIRST,
  MERGED_OUTSIDE,
  isDirty,
  useRecipeEditorStore,
} from './use-recipe-editor-store';

const REACH = 'reach-for-the-lantern';
const store = () => useRecipeEditorStore.getState();
const layers = () => store().editing?.draft.layers ?? [];

/** An agent's update_recipe: the file changes, then the event comes. */
async function agentWrites(change: (l: RecipeLayer[]) => RecipeLayer[], event = true) {
  const doc = await commands.recipeGet('frostveil', REACH);
  await commands.recipeUpdate('frostveil', REACH, {
    name: null,
    layers: change(doc.recipe.layers),
    blanks: null,
  });
  if (event) await store().onChanged('frostveil');
}

const weigh = (id: string, weight: number) => (l: RecipeLayer[]) =>
  l.map(x => (x.id === id ? { ...x, settings: { ...x.settings, weight } } : x));

const editLayers = (change: (l: RecipeLayer[]) => RecipeLayer[]) =>
  store().edit(d => ({ ...d, layers: change(d.layers) }));

beforeEach(async () => {
  await openMockLibrary('demo');
  await store().open('frostveil', REACH);
});

afterEach(() => {
  vi.restoreAllMocks();
  clearMocks();
  useRecipeEditorStore.setState(useRecipeEditorStore.getInitialState(), true);
});

describe('the recipe editor store', () => {
  it('opens a recipe, edits it and saves the changes whole, leaving the blanks alone', async () => {
    expect(layers().map(l => l.kind)).toEqual(['reach', 'look-at', 'wave']);
    expect(isDirty(store().editing)).toBe(false);
    editLayers(weigh('l-1', 0.5));
    expect(isDirty(store().editing)).toBe(true);
    const update = vi.spyOn(commands, 'recipeUpdate');
    expect(await store().save()).toBe(true);
    expect(update.mock.calls[0]?.[2]).toMatchObject({ blanks: null });
    expect(update.mock.calls[0]?.[2].layers?.[0]?.settings.weight).toBe(0.5);
    expect(isDirty(store().editing)).toBe(false);
  });

  it('refuses to save what the core would refuse, and says to fix it first', async () => {
    store().edit(d => ({ ...d, layers: [...d.layers, newLayer('reach', d.layers, [])] }));
    store().edit(d => ({
      ...d,
      layers: d.layers.map(l =>
        l.id === 'l-4' ? { ...l, settings: { ...l.settings, target: null } } : l
      ),
    }));
    const update = vi.spyOn(commands, 'recipeUpdate');
    expect(await store().save()).toBe(false);
    expect(store().saveError).toBe(FIX_FIRST);
    expect(update).not.toHaveBeenCalled();
  });

  it('asks before leaving unsaved edits, and drops them only when told to', async () => {
    editLayers(weigh('l-1', 0.3));
    expect(await store().leave({ to: 'list' })).toBe(false);
    expect(store().leaving).toEqual({ to: 'list' });
    store().stay();
    expect(store().editing?.draft.layers[0]?.settings.weight).toBe(0.3);
    await store().leave({ to: 'list' });
    await store().leaveAnyway();
    expect(store().editing).toBeNull();
    expect((await commands.recipeGet('frostveil', REACH)).recipe.layers[0]?.settings.weight).toBe(
      1
    );
  });

  it('saves on the way to another recipe when asked, then opens it', async () => {
    editLayers(weigh('l-1', 0.3));
    await store().open('frostveil', 'look-around');
    expect(store().leaving).toEqual({ to: 'recipe', id: 'look-around' });
    expect(store().editing?.id).toBe(REACH);
    await store().saveAndLeave();
    expect(store().editing?.id).toBe('look-around');
    expect((await commands.recipeGet('frostveil', REACH)).recipe.layers[0]?.settings.weight).toBe(
      0.3
    );
  });

  it('shows an outside change as it is when nothing is unsaved', async () => {
    await agentWrites(weigh('l-2', 0.1));
    expect(store().merged).toBe(CHANGED_OUTSIDE);
    expect(layers()[1]?.settings.weight).toBe(0.1);
    expect(isDirty(store().editing)).toBe(false);
  });

  it('merges an outside change that does not clash with the unsaved edits', async () => {
    editLayers(weigh('l-1', 0.4));
    await agentWrites(weigh('l-2', 0.1));
    expect(store().merged).toBe(MERGED_OUTSIDE);
    expect(layers().map(l => l.settings.weight)).toEqual([0.4, 0.1, 1]);
    expect(await store().save()).toBe(true);
    const saved = (await commands.recipeGet('frostveil', REACH)).recipe.layers;
    expect(saved.map(l => l.settings.weight)).toEqual([0.4, 0.1, 1]);
  });

  it('asks Keep mine or Use theirs on a clash, and saves nothing until answered', async () => {
    editLayers(weigh('l-1', 0.4));
    await agentWrites(weigh('l-1', 0.9));
    expect(store().outside?.merge.clashes).toEqual(['change l-1']);
    expect(await store().save()).toBe(false);
    expect(store().saveError).toBe(ANSWER_FIRST);
    store().keepMine();
    expect(layers()[0]?.settings.weight).toBe(0.4);
    expect(await store().save()).toBe(true);
    expect((await commands.recipeGet('frostveil', REACH)).recipe.layers[0]?.settings.weight).toBe(
      0.4
    );
  });

  it('takes theirs on Use theirs', async () => {
    editLayers(weigh('l-1', 0.4));
    await agentWrites(weigh('l-1', 0.9));
    store().useTheirs();
    expect(layers()[0]?.settings.weight).toBe(0.9);
    expect(isDirty(store().editing)).toBe(false);
  });

  it('reads the file before saving, so a write whose event has not come stays', async () => {
    editLayers(weigh('l-1', 0.4));
    await agentWrites(weigh('l-3', 0.2), false);
    expect(await store().save()).toBe(true);
    const saved = (await commands.recipeGet('frostveil', REACH)).recipe.layers;
    expect(saved.map(l => l.settings.weight)).toEqual([0.4, 0.8, 0.2]);
  });

  it('keeps a gone prop stored when the blanks are saved with it open', async () => {
    await store().open('frostveil', 'look-around');
    expect(store().editing?.doc.warnings[0]).toMatch(/old-torch/);
    store().edit(d => ({ ...d, blanks: d.blanks.map(b => ({ ...b, label: 'The wall torch' })) }));
    expect(await store().save()).toBe(true);
    expect(store().editing?.doc.warnings[0]).toMatch(/old-torch/);
  });

  it('saves for quitting, or says why it could not', async () => {
    expect(await store().saveForQuit()).toBeNull();
    editLayers(weigh('l-1', 0.6));
    expect(await store().saveForQuit()).toBeNull();
    store().edit(d => ({
      ...d,
      layers: [...d.layers, { ...newLayer('nudge', d.layers, []), bone: 'tail' }],
    }));
    expect(await store().saveForQuit()).toBe(FIX_FIRST);
  });
});
