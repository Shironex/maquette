import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, type RecipeLayer } from '@/ipc/client';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import { newLayer } from './recipe-draft';
import {
  ANSWER_FIRST,
  CHANGED_OUTSIDE,
  FIX_FIRST,
  MERGED_OUTSIDE,
  MOVED,
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
    expect(update.mock.calls[0]?.[2]).toMatchObject({ blanks: null, clear: [] });
    expect(update.mock.calls[0]?.[2].layers?.[0]?.settings.weight).toBe(0.5);
    expect(update.mock.calls[0]?.[2].expected).toBeTruthy();
    expect(isDirty(store().editing)).toBe(false);
  });

  it('refuses to save what the core would refuse, and says to fix it first', async () => {
    const added = { ...newLayer('reach', layers(), []), settings: { ...layers()[0]!.settings } };
    store().edit(d => ({
      ...d,
      layers: [...d.layers, { ...added, settings: { ...added.settings, target: null } }],
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
    expect(store().merged).toBeNull();
  });

  it('drops the merged note on Discard, and once a resolved clash is saved', async () => {
    editLayers(weigh('l-1', 0.4));
    await agentWrites(weigh('l-2', 0.1));
    expect(store().merged).toBe(MERGED_OUTSIDE);
    store().discard();
    expect(store().merged).toBeNull();

    editLayers(weigh('l-1', 0.4));
    await agentWrites(weigh('l-3', 0.2));
    await agentWrites(weigh('l-1', 0.9));
    expect(store().outside).not.toBeNull();
    store().useTheirs();
    expect(store().merged).toBe(MERGED_OUTSIDE);
    expect(await store().save()).toBe(true);
    expect(store().merged).toBeNull();
  });

  it('drops the note of an outside change once Save finds nothing left to write', async () => {
    await agentWrites(weigh('l-2', 0.1));
    expect(store().merged).toBe(CHANGED_OUTSIDE);
    expect(await store().save()).toBe(true);
    expect(store().merged).toBeNull();
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

  it('sends only the blanks when only the blanks changed', async () => {
    store().edit(d => ({ ...d, blanks: d.blanks.map(b => ({ ...b, label: `${b.label}!` })) }));
    const update = vi.spyOn(commands, 'recipeUpdate');
    expect(await store().save()).toBe(true);
    expect(update.mock.calls[0]?.[2]).toMatchObject({ layers: null });
    expect(update.mock.calls[0]?.[2].blanks).toHaveLength(2);
  });

  it('never overwrites a write that lands after the reread: it merges it, then saves again', async () => {
    editLayers(weigh('l-1', 0.4));
    const realGet = commands.recipeGet.bind(commands);
    let first = true;
    vi.spyOn(commands, 'recipeGet').mockImplementation(async (project, id) => {
      const doc = await realGet(project, id);
      if (first) {
        first = false;
        // The agent writes just after Save read the file.
        await agentWrites(weigh('l-3', 0.2), false);
      }
      return doc;
    });
    expect(await store().save()).toBe(false);
    expect(store().saveError).toBe(MOVED);
    let saved = (await realGet('frostveil', REACH)).recipe.layers;
    expect(saved.map(l => l.settings.weight)).toEqual([1, 0.8, 0.2]);
    expect(layers().map(l => l.settings.weight)).toEqual([0.4, 0.8, 0.2]);
    expect(await store().save()).toBe(true);
    saved = (await realGet('frostveil', REACH)).recipe.layers;
    expect(saved.map(l => l.settings.weight)).toEqual([0.4, 0.8, 0.2]);
  });

  it('asks only on a real clash when a write lands after the reread', async () => {
    editLayers(weigh('l-1', 0.4));
    const realGet = commands.recipeGet.bind(commands);
    let first = true;
    vi.spyOn(commands, 'recipeGet').mockImplementation(async (project, id) => {
      const doc = await realGet(project, id);
      if (first) {
        first = false;
        await agentWrites(weigh('l-1', 0.9), false);
      }
      return doc;
    });
    expect(await store().save()).toBe(false);
    expect(store().outside?.merge.clashes).toEqual(['change l-1']);
    expect((await realGet('frostveil', REACH)).recipe.layers[0]?.settings.weight).toBe(0.9);
  });

  it('drops a pending clash once a later outside change merges cleanly', async () => {
    editLayers(weigh('l-1', 0.4));
    await agentWrites(weigh('l-1', 0.9));
    expect(store().outside).not.toBeNull();
    // The agent puts l-1 back and changes l-3 instead.
    await agentWrites(l => weigh('l-3', 0.2)(weigh('l-1', 1)(l)));
    expect(store().outside).toBeNull();
    expect(store().merged).toBe(MERGED_OUTSIDE);
    store().keepMine();
    expect(layers().map(l => l.settings.weight)).toEqual([0.4, 0.8, 0.2]);
  });

  it('empties a gone prop for good only when told to', async () => {
    await store().open('frostveil', 'look-around');
    expect(store().editing?.doc.gone).toEqual([{ blank: 'torch', prop: 'old-torch' }]);
    store().clearGone('torch', true);
    expect(isDirty(store().editing)).toBe(true);
    const update = vi.spyOn(commands, 'recipeUpdate');
    expect(await store().save()).toBe(true);
    expect(update.mock.calls[0]?.[2]).toMatchObject({ clear: ['torch'] });
    expect(update.mock.calls[0]?.[2].blanks).toHaveLength(1);
    expect(store().editing?.doc.gone).toEqual([]);
    expect(store().editing?.cleared).toEqual([]);
    expect(isDirty(store().editing)).toBe(false);
  });

  it('never sends a clear for a gone prop blank removed after Leave open, and saves', async () => {
    await store().open('frostveil', 'look-around');
    store().clearGone('torch', true);
    store().edit(d => ({
      ...d,
      blanks: [],
      layers: d.layers.map(l => ({ ...l, settings: { ...l.settings, target: null } })),
    }));
    store().edit(d => ({ ...d, layers: [] }));
    const update = vi.spyOn(commands, 'recipeUpdate');
    expect(await store().save()).toBe(true);
    expect(update.mock.calls[0]?.[2]).toMatchObject({ clear: [], blanks: [] });
    expect(isDirty(store().editing)).toBe(false);
    expect(store().editing?.cleared).toEqual([]);
  });

  it('never sends a clear for a gone prop blank turned into a point after Leave open', async () => {
    await store().open('frostveil', 'look-around');
    store().clearGone('torch', true);
    store().edit(d => ({
      ...d,
      blanks: d.blanks.map(b => ({ ...b, kind: 'point' as const, prop: null, point: [0, 1, 1] })),
    }));
    const update = vi.spyOn(commands, 'recipeUpdate');
    expect(await store().save()).toBe(true);
    expect(update.mock.calls[0]?.[2]).toMatchObject({ clear: [] });
    expect(isDirty(store().editing)).toBe(false);
  });

  it('recovers from a clear left over for a blank that is gone from the draft', async () => {
    await store().open('frostveil', 'look-around');
    editLayers(l => l.map(x => ({ ...x, settings: { ...x.settings, weight: 0.5 } })));
    const editing = store().editing!;
    useRecipeEditorStore.setState({ editing: { ...editing, cleared: ['torch', 'nothing'] } });
    store().edit(d => ({
      ...d,
      blanks: d.blanks.map(b => ({ ...b, kind: 'point' as const, prop: null, point: [0, 1, 1] })),
    }));
    expect(await store().save()).toBe(true);
    expect(store().saveError).toBeNull();
    expect(isDirty(store().editing)).toBe(false);
  });

  it('keeps the settings open on a blank of mine the merge renamed', async () => {
    const mine = {
      id: 'prop-k3x9q',
      label: 'Mine',
      kind: 'point' as const,
      prop: null,
      point: [0, 1, 1] as [number, number, number],
    };
    store().edit(d => ({ ...d, blanks: [...d.blanks, mine] }));
    store().pick({ what: 'blank', id: mine.id });
    const doc = await commands.recipeGet('frostveil', REACH);
    await commands.recipeUpdate('frostveil', REACH, {
      name: null,
      layers: null,
      blanks: [...doc.recipe.blanks, { ...mine, label: 'Theirs', point: [1, 1, 1] }],
    });
    await store().onChanged('frostveil');
    const renamed = store().editing?.draft.blanks.find(b => b.label === 'Mine')?.id;
    expect(renamed).toMatch(/^point-[a-z0-9]{5}$/);
    expect(renamed).not.toBe(mine.id);
    expect(store().picked).toEqual({ what: 'blank', id: renamed });
  });

  it('waits for a Save already running when the window closes', async () => {
    editLayers(weigh('l-1', 0.6));
    const saving = store().save();
    const quitting = store().saveForQuit();
    expect(await quitting).toBeNull();
    expect(await saving).toBe(true);
    expect((await commands.recipeGet('frostveil', REACH)).recipe.layers[0]?.settings.weight).toBe(
      0.6
    );
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

  it('reads gone only for a file that is not there, and clears it on a later good read', async () => {
    const realGet = commands.recipeGet.bind(commands);
    const get = vi.spyOn(commands, 'recipeGet');
    get.mockRejectedValueOnce({ code: 'json', message: 'expected value at line 1' });
    await store().onChanged('frostveil');
    expect(store().gone).toBe(false);
    expect(store().unreadable).toBe(true);
    get.mockRejectedValueOnce(new Error('IPC went away'));
    editLayers(weigh('l-1', 0.4));
    await store().onChanged('frostveil');
    expect(store().unreadable).toBe(true);
    expect(layers()[0]?.settings.weight).toBe(0.4);
    get.mockImplementation(realGet);
    await store().onChanged('frostveil');
    expect(store().unreadable).toBe(false);

    get.mockRejectedValueOnce({ code: 'not-found', message: 'Not found: the recipe file' });
    await store().onChanged('frostveil');
    expect(store().gone).toBe(true);
    expect(store().unreadable).toBe(false);
    await store().onChanged('frostveil');
    expect(store().gone).toBe(false);
    expect(layers()[0]?.settings.weight).toBe(0.4);
  });

  it('reads gone when the mock backend no longer has the recipe, as the core answers', async () => {
    await commands.recipeRemove('frostveil', REACH);
    await store().onChanged('frostveil');
    expect(store().gone).toBe(true);
  });
});
