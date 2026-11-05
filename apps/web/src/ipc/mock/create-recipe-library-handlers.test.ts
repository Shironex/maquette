import { describe, expect, it } from 'vitest';
import type {
  LibraryRecipeDoc,
  LibraryRecipeSummary,
  McpStatus,
  RecipeCompare,
  RecipeImported,
  RecipeSavePlan,
  RecipeSaved,
  RecipeSummary,
  RecipeUsed,
} from '../client';
import { createBackend } from './backend';

function backend(library: string | null = 'demo', agents: string | null = null) {
  return createBackend({
    scenario: 'configured',
    probeDelay: 0,
    pace: 0,
    library,
    agents,
    emit: () => {},
  });
}

type Handle = ReturnType<typeof backend>;

const list = async (handle: Handle) =>
  (await handle('recipe_library_list')) as LibraryRecipeSummary[];
const plan = async (handle: Handle, recipe: string, entry: string | null) =>
  (await handle('recipe_library_plan_save', {
    project: 'frostveil',
    recipe,
    entry,
  })) as RecipeSavePlan;
const saveAs = (handle: Handle, p: RecipeSavePlan) =>
  handle('recipe_library_save', {
    save: {
      project: p.project,
      recipe: p.recipe,
      entry: p.entry,
      expected: p.entryVersion,
      recipeExpected: p.recipeVersion,
    },
  }) as Promise<RecipeSaved>;

describe('the mock recipe library', () => {
  it('opens empty without the demo, and with three entries with it', async () => {
    expect(await list(backend(null))).toEqual([]);
    const entries = await list(backend());
    expect(entries.map(e => [e.id, e.latest, e.revisions.length])).toEqual([
      ['guard-and-look', 1, 1],
      ['reach-for-the-lantern', 3, 3],
      ['wave-hello', 1, 1],
    ]);
    expect(entries.find(e => e.id === 'wave-hello')?.revisions[0]?.imported).toBe(
      'wave-hello.recipe.json'
    );
    expect(entries.find(e => e.id === 'guard-and-look')?.mixamo).toBe(true);
  });

  it('says the lantern recipe came from r1 and what r3 changes against it', async () => {
    const handle = backend();
    const recipes = (await handle('recipe_list', { project: 'frostveil' })) as RecipeSummary[];
    expect(recipes.find(r => r.id === 'reach-for-the-lantern')?.from).toEqual({
      entry: 'reach-for-the-lantern',
      revision: 1,
    });
    const compare = (await handle('recipe_library_compare', {
      project: 'frostveil',
      recipe: 'reach-for-the-lantern',
    })) as RecipeCompare;
    expect(compare).toMatchObject({ revision: 1, latest: 3 });
    expect(compare.changes).toEqual([
      'Edits the wave on leftLowerArm (l-3): cycles.',
      'Adds the nudge on spine (l-4).',
    ]);
  });

  it('saves a new revision and never overwrites one', async () => {
    const handle = backend();
    const first = await plan(handle, 'reach-for-the-lantern', 'reach-for-the-lantern');
    expect(first).toMatchObject({ latest: 3, revision: 4, same: false });
    const saved = await saveAs(handle, first);
    expect(saved).toMatchObject({ entry: 'reach-for-the-lantern', revision: 4, linked: true });
    const r3 = (await handle('recipe_library_get', {
      entry: 'reach-for-the-lantern',
      revision: 3,
    })) as LibraryRecipeDoc;
    expect(r3.recipe.layers).toHaveLength(4);
    const r4 = (await handle('recipe_library_get', {
      entry: 'reach-for-the-lantern',
      revision: null,
    })) as LibraryRecipeDoc;
    expect([r4.revision, r4.recipe.layers.length]).toEqual([4, 3]);
    const again = await plan(handle, 'reach-for-the-lantern', 'reach-for-the-lantern');
    expect(again).toMatchObject({ same: true, changes: [] });
    await expect(saveAs(handle, again)).rejects.toMatchObject({ code: 'invalid' });
  });

  it('refuses a save against an entry or a recipe that moved since the plan', async () => {
    const handle = backend();
    const stale = await plan(handle, 'reach-for-the-lantern', 'reach-for-the-lantern');
    const other = await plan(handle, 'look-around', 'reach-for-the-lantern');
    await saveAs(handle, other);
    await expect(saveAs(handle, stale)).rejects.toMatchObject({ code: 'moved' });
    await handle('recipe_rename', { project: 'frostveil', id: 'look-around', name: 'Look up' });
    const moved = { ...other, entryVersion: (await list(handle))[1]!.version };
    await expect(saveAs(handle, moved)).rejects.toMatchObject({ code: 'moved' });
  });

  it('saves a new library recipe under a fresh id and closes the agent proposal', async () => {
    const handle = backend('demo', 'recipe-save');
    const waiting = async () =>
      ((await handle('mcp_status')) as McpStatus).approvals.filter(a => a.kind === 'recipe-save');
    expect(await waiting()).toHaveLength(1);
    const fresh = await plan(handle, 'reach-for-the-lantern', null);
    expect(fresh.changes).toHaveLength(1);
    expect(fresh.changes[0]).toMatch(/^A new library recipe on .+ with 3 changes and 2 blanks\.$/);
    const saved = await saveAs(handle, fresh);
    expect(saved).toMatchObject({ entry: 'reach-for-the-lantern-2', revision: 1 });
    expect(await waiting()).toEqual([]);
  });

  it('uses a revision under a fresh id and notes what to know', async () => {
    const handle = backend();
    const used = (await handle('recipe_library_use', {
      request: { project: 'frostveil', entry: 'reach-for-the-lantern', revision: 2, name: null },
    })) as RecipeUsed;
    expect(used.doc.id).toBe('reach-for-the-lantern-2');
    expect(used.doc.recipe.from).toEqual({ entry: 'reach-for-the-lantern', revision: 2 });
    expect(used.notes).toContain(
      'The project has a recipe reach-for-the-lantern already, so this one is reach-for-the-lantern-2.'
    );
  });

  it('imports the canned file with a hash warning and an opened prop', async () => {
    const handle = backend();
    const used = (await handle('recipe_import_to_project', { project: 'frostveil' })) as RecipeUsed;
    expect(used.doc.recipe.name).toBe('Sit and wave');
    expect(used.notes.join(' ')).toMatch(/hash differs/);
    expect(used.notes.join(' ')).toMatch(/stone-bench, which Frostveil does not have/);
    const first = (await handle('recipe_import_to_library')) as RecipeImported;
    const second = (await handle('recipe_import_to_library')) as RecipeImported;
    expect([first.entry, second.entry]).toEqual(['sit-and-wave', 'sit-and-wave-2']);
    expect(second.notes[0]).toMatch(/already, so this one is sit-and-wave-2/);
  });

  it('removes a revision but never the only one, and refuses a stale version', async () => {
    const handle = backend();
    const [, reach] = await list(handle);
    await expect(
      handle('recipe_library_remove_revision', {
        entry: 'reach-for-the-lantern',
        revision: 2,
        expected: 'stale',
      })
    ).rejects.toMatchObject({ code: 'moved' });
    await handle('recipe_library_remove_revision', {
      entry: 'reach-for-the-lantern',
      revision: 2,
      expected: reach!.version,
    });
    await expect(
      handle('recipe_library_remove_revision', {
        entry: 'wave-hello',
        revision: 1,
        expected: null,
      })
    ).rejects.toMatchObject({ code: 'invalid' });
    await handle('recipe_library_remove', { entry: 'wave-hello', expected: null });
    expect((await list(handle)).map(e => [e.id, e.revisions.map(r => r.revision)])).toEqual([
      ['guard-and-look', [1]],
      ['reach-for-the-lantern', [1, 3]],
    ]);
  });
});
