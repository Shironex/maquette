import { afterEach, describe, expect, it } from 'vitest';
import { commands } from '@/ipc/client';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import { useGripLibraryStore } from './use-grip-library-store';
import { gripOfSheet, useGripSaveStore } from './use-grip-save-store';
import { useRecipeEditorStore } from './use-recipe-editor-store';

afterEach(() => {
  clearMocks();
  useGripSaveStore.setState(useGripSaveStore.getInitialState(), true);
  useGripLibraryStore.setState(useGripLibraryStore.getInitialState(), true);
});

const store = () => useGripSaveStore.getState();
const sheet = () => store().sheet!;
const open = () => store().open('frostveil', 'scythe-charge-held', 'l-1', 30);

describe('the save a grip sheet', () => {
  it('reads both hands at the frame and plans a new grip named after the recipe', async () => {
    await openRecipe('scythe-charge-held');
    await open();
    expect(Object.keys(sheet().pose ?? {})).toHaveLength(33);
    expect(sheet()).toMatchObject({ hands: 'both', name: 'Scythe charge, held', entry: null });
    expect(sheet().plan).toMatchObject({ entry: null, revision: 1, same: false });
    expect(sheet().plan?.file).toMatch(/^library\/grips\/.+\/r1\.grip\.json$/);
    expect(sheet().plan?.grip.hands).toEqual({ holds: 'right', supports: 'left' });
  });

  it('keeps one hand and the bones asked for, and plans again on each choice', async () => {
    await openRecipe('scythe-charge-held');
    await open();
    await store().choose({ hands: 'left', weapon: 'shield' });
    expect(Object.keys(sheet().plan?.grip.pose ?? {}).every(b => b.startsWith('left'))).toBe(true);
    expect(sheet().plan?.grip).toMatchObject({ weapon: 'shield', hands: { holds: 'left' } });
    await store().choose({ bones: sheet().bones.filter(b => !b.includes('Thumb')) });
    expect(Object.keys(sheet().plan?.grip.pose ?? {}).some(b => b.includes('Thumb'))).toBe(false);
    await store().choose({ name: '  ' });
    expect(sheet().plan).toBeNull();
    expect(gripOfSheet({ ...sheet(), bones: [] })).toBeNull();
  });

  it('reads unsaved keys from the editor while the recipe is open there', async () => {
    await openRecipe('scythe-charge-held');
    useRecipeEditorStore.getState().edit(d => ({
      ...d,
      layers: d.layers.map(l =>
        l.id === 'l-1'
          ? {
              ...l,
              keys: l.keys?.map(k => ({
                ...k,
                pose: Object.fromEntries(
                  Object.entries(k.pose).map(([b, t]) => [b, b === 'rightHand' ? [1, 2, 3] : t])
                ),
              })),
            }
          : l
      ),
    }));
    await open();
    expect(sheet().pose?.rightHand).toEqual([1, 2, 3]);
  });

  it('saves a revision of a library grip, taking its name, and says so', async () => {
    await openRecipe('scythe-charge-held');
    await open();
    await store().choose({ entry: 'scythe-staff-grip' });
    expect(sheet()).toMatchObject({ name: 'Scythe, staff grip', weapon: 'scythe' });
    expect(sheet().plan).toMatchObject({ entry: 'scythe-staff-grip', revision: 3, latest: 2 });
    expect(sheet().plan?.changes).toEqual([expect.stringMatching(/another way/)]);
    expect(await store().save()).toBe(true);
    expect(store().sheet).toBeNull();
    const doc = await commands.gripLibraryGet('scythe-staff-grip', null);
    expect(doc.revision).toBe(3);
    expect(doc.entry.revisions.at(-1)).toMatchObject({
      project: 'frostveil',
      recipe: 'scythe-charge-held',
      layer: 'l-1',
      frame: 30,
    });
    expect(useGripLibraryStore.getState().list?.find(g => g.id === doc.entry.id)?.latest).toBe(3);
  });

  it('plans again and says so when the library grip moved since it planned', async () => {
    await openRecipe('scythe-charge-held');
    await open();
    await store().choose({ entry: 'scythe-staff-grip' });
    const planned = sheet().plan!;
    await commands.gripLibrarySave({
      grip: { ...planned.grip, weapon: 'staff' },
      entry: 'scythe-staff-grip',
      expected: planned.entryVersion,
      source: null,
    });
    expect(await store().save()).toBe(false);
    expect(sheet().moved).toBe(true);
    expect(sheet().plan?.revision).toBe(4);
  });

  it('opens as an agent proposed it', async () => {
    await openRecipe('scythe-charge-held');
    await store().show('frostveil', 'scythe-charge-held', 'scythe-staff-grip', {
      layer: 'l-1',
      frame: 30,
      name: 'Scythe, staff grip',
      weapon: 'scythe',
      hands: { holds: 'right', supports: null },
      bones: ['rightHand', 'rightIndexProximal', 'chest'],
      latest: 2,
      file: 'library/grips/scythe-staff-grip/r3.grip.json',
    });
    expect(sheet()).toMatchObject({
      hands: 'right',
      entry: 'scythe-staff-grip',
      bones: ['rightHand', 'rightIndexProximal'],
    });
    expect(sheet().plan?.revision).toBe(3);
  });

  it('says when the change turns no hand at the frame', async () => {
    await openRecipe('scythe-charge-held');
    await store().open('frostveil', 'scythe-charge-held', 'l-2', 30);
    expect(sheet().error).toMatch(/turn no hand at frame 30/);
  });
});
