import { describe, expect, it, vi } from 'vitest';
import type {
  GripFile,
  GripSavePlan,
  GripSaved,
  LibraryGripDoc,
  LibraryGripEntry,
  LibraryGripSummary,
  McpStatus,
  RecipeBakeRecord,
  RecipeDoc,
  RecipeSummary,
} from '../client';
import { gripOf, keyedPose } from '@/state/recipe-grips';
import { timeline } from '@/state/recipe-timeline';
import { createBackend } from './backend';
import { gripChanges } from './create-grip-library-handlers';
import { GRIP_DEMO, SCYTHE_UID } from './grip-library-fixtures';

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

const list = async (handle: Handle) => (await handle('grip_library_list')) as LibraryGripSummary[];
const get = async (handle: Handle, entry: string, revision: number | null = null) =>
  (await handle('grip_library_get', { entry, revision })) as LibraryGripDoc;
const plan = async (handle: Handle, grip: GripFile, entry: string | null) =>
  (await handle('grip_library_plan_save', { grip, entry })) as GripSavePlan;
const source = { project: 'frostveil', recipe: GRIP_DEMO, layer: 'l-1', frame: 30 };
const saveAs = (handle: Handle, p: GripSavePlan, from: typeof source | null = source) =>
  handle('grip_library_save', {
    save: { grip: p.grip, entry: p.entry, expected: p.entryVersion, source: from },
  }) as Promise<GripSaved>;

/** The hand pose the demo recipe's pose keys hold at frame 30, as the save sheet reads it. */
async function keyed(handle: Handle): Promise<GripFile> {
  const doc = (await handle('recipe_get', { project: 'frostveil', id: GRIP_DEMO })) as RecipeDoc;
  const layer = doc.recipe.layers.find(l => l.id === 'l-1')!;
  const pose = keyedPose(doc.recipe, timeline(doc.recipe), layer, 30)!;
  const made = gripOf(pose)!;
  return { version: 1, name: 'Scythe, staff grip', revision: 1, weapon: 'scythe', ...made };
}

describe('the mock grip library', () => {
  it('opens empty without the demo, and with six grips with it', async () => {
    expect(await list(backend(null))).toEqual([]);
    const grips = await list(backend());
    expect(grips.map(g => [g.id, g.latest, g.group, g.handsWords])).toEqual([
      ['dagger-reverse', 1, 'daggers', 'right hand'],
      ['empty-hand-relaxed', 4, 'empty', 'either hand'],
      ['longsword-two-hands', 3, 'swords', 'both hands'],
      ['rapier-sabre-grip', 1, 'swords', 'right hand'],
      ['scythe-staff-grip', 2, 'polearms', 'both hands'],
      ['shield-strap', 1, 'shields', 'left hand'],
    ]);
    const longsword = grips.find(g => g.id === 'longsword-two-hands')!;
    expect(Object.keys(longsword.pose)).toHaveLength(33);
    expect(longsword.problem).toBeNull();
  });

  it('reads a revision, the newest unless told, and refuses one it does not list', async () => {
    const handle = backend();
    const newest = await get(handle, 'scythe-staff-grip');
    expect(newest).toMatchObject({ revision: 2, grip: { weapon: 'scythe', revision: 2 } });
    expect(newest.entry.uid).toBe(SCYTHE_UID);
    const first = await get(handle, 'scythe-staff-grip', 1);
    expect(first.grip.pose.rightIndexProximal).not.toEqual(newest.grip.pose.rightIndexProximal);
    await expect(get(handle, 'scythe-staff-grip', 7)).rejects.toMatchObject({ code: 'not-found' });
    await expect(get(handle, 'no-such-grip')).rejects.toMatchObject({ code: 'not-found' });
  });

  it('lists the demo recipe with its grip copied from r1, while the library has r2', async () => {
    const handle = backend();
    const recipes = (await handle('recipe_list', { project: 'frostveil' })) as RecipeSummary[];
    expect(recipes.find(r => r.id === GRIP_DEMO)?.grips).toEqual([
      { layer: 'l-2', from: { entry: 'scythe-staff-grip', uid: SCYTHE_UID, revision: 1 } },
    ]);
    expect(recipes.find(r => r.id === 'look-around')?.grips).toEqual([]);
    const doc = (await handle('recipe_get', { project: 'frostveil', id: GRIP_DEMO })) as RecipeDoc;
    const held = doc.recipe.layers.find(l => l.kind === 'grip')!;
    expect(held.pose).toEqual((await get(handle, 'scythe-staff-grip', 1)).grip.pose);
    expect(held.settings).toMatchObject({ start: 0.1, end: 0.9, easeIn: 4, easeOut: 4 });
  });

  it('plans a save as a new grip and as the next revision, and says when nothing changed', async () => {
    const handle = backend();
    const grip = await keyed(handle);
    expect(Object.keys(grip.pose)).toHaveLength(33);
    const fresh = await plan(handle, { ...grip, name: 'Scythe, high guard' }, null);
    expect(fresh).toMatchObject({
      entry: null,
      entryName: 'Scythe, high guard',
      latest: null,
      revision: 1,
      file: 'library/grips/scythe-high-guard/r1.grip.json',
      changes: ['A new grip for both hands: 33 bones.'],
      same: false,
    });
    expect(fresh.bytes).toBeGreaterThan(600);
    const taken = await plan(handle, grip, null);
    expect(taken.entryName).toBe('Scythe, staff grip (2)');
    expect(taken.file).toBe('library/grips/scythe-staff-grip-2/r1.grip.json');
    // The recipe's keys hold what r2 holds, so a save to that grip has nothing to add.
    const same = await plan(handle, grip, 'scythe-staff-grip');
    expect(same).toMatchObject({ latest: 2, revision: 3, same: true, changes: [] });
    const turned: GripFile = { ...grip, pose: { ...grip.pose, rightIndexProximal: [0, 0, 80] } };
    const next = await plan(handle, { ...turned, name: 'Ignored' }, 'scythe-staff-grip');
    expect(next).toMatchObject({
      entry: 'scythe-staff-grip',
      entryName: 'Scythe, staff grip',
      revision: 3,
      file: 'library/grips/scythe-staff-grip/r3.grip.json',
      changes: ['Turns 1 bone another way.'],
      same: false,
    });
    expect(next.grip.name).toBe('Scythe, staff grip');
  });

  it('saves the next revision, keeps where it came from and closes the agent proposal', async () => {
    const handle = backend('demo', 'grip-save');
    const waiting = async () =>
      ((await handle('mcp_status')) as McpStatus).approvals.filter(a => a.kind === 'grip-save');
    expect((await waiting())[0]?.recipeSave).toMatchObject({
      recipe: GRIP_DEMO,
      entry: 'scythe-staff-grip',
      revision: 3,
      grip: { layer: 'l-1', frame: 30, latest: 2 },
    });
    const grip = await keyed(handle);
    const turned: GripFile = { ...grip, pose: { ...grip.pose, rightIndexProximal: [0, 0, 80] } };
    const planned = await plan(handle, turned, 'scythe-staff-grip');
    const saved = await saveAs(handle, planned);
    expect(saved).toMatchObject({ entry: 'scythe-staff-grip', revision: 3 });
    expect(saved.entryVersion).not.toBe(planned.entryVersion);
    const after = await get(handle, 'scythe-staff-grip');
    expect(after.revision).toBe(3);
    expect(after.entry.revisions.at(-1)).toMatchObject({ revision: 3, ...source });
    expect(await waiting()).toEqual([]);
    // The plan read the grip before that save, so the same save again is refused as moved.
    await expect(saveAs(handle, planned)).rejects.toMatchObject({ code: 'moved' });
    const again = await plan(handle, turned, 'scythe-staff-grip');
    await expect(saveAs(handle, again)).rejects.toMatchObject({
      code: 'invalid',
      message: expect.stringMatching(/nothing to save/),
    });
  });

  it('saves a new grip under a fresh id and a name no other grip has', async () => {
    const handle = backend();
    const grip = await keyed(handle);
    const saved = await saveAs(handle, await plan(handle, grip, null), null);
    expect(saved).toMatchObject({
      entry: 'scythe-staff-grip-2',
      name: 'Scythe, staff grip (2)',
      revision: 1,
    });
    const made = await get(handle, saved.entry);
    expect(made.entry.uid).toMatch(/^[0-9a-f]{16}$/);
    expect(made.entry.uid).not.toBe(SCYTHE_UID);
    expect(made.entry.revisions[0]).toMatchObject({ project: null, layer: null, frame: null });
    expect(await list(handle)).toHaveLength(7);
  });

  it('refuses what the core refuses in a grip', async () => {
    const handle = backend();
    const grip = await keyed(handle);
    const refused = (over: Partial<GripFile>) => plan(handle, { ...grip, ...over }, null);
    await expect(refused({ name: '  ' })).rejects.toMatchObject({
      message: 'Give the grip a name.',
    });
    await expect(refused({ weapon: 'lightsaber' })).rejects.toMatchObject({
      message: expect.stringMatching(/^"lightsaber" is not a weapon a grip holds\. Use one of /),
    });
    await expect(refused({ pose: {} })).rejects.toMatchObject({
      message: expect.stringMatching(/turns no bone/),
    });
    await expect(refused({ pose: { ...grip.pose, chest: [0, 10, 0] } })).rejects.toMatchObject({
      message: 'A grip poses a hand, its fingers and the arm above it, not chest.',
    });
    await expect(refused({ pose: { ...grip.pose, rightHand: [0, 0, 200] } })).rejects.toMatchObject(
      { message: expect.stringMatching(/more than 180 degrees/) }
    );
    await expect(refused({ hands: { holds: 'right', supports: null } })).rejects.toMatchObject({
      message: expect.stringMatching(/poses both hands/),
    });
    await expect(
      handle('grip_library_save', {
        save: { grip, entry: 'scythe-staff-grip', expected: null, source: null },
      })
    ).rejects.toMatchObject({ message: expect.stringMatching(/needs the version its plan read/) });
  });

  it('removes a revision but not the last one, and a whole grip, each checked against a move', async () => {
    const handle = backend();
    const read = await get(handle, 'longsword-two-hands');
    await expect(
      handle('grip_library_remove_revision', {
        entry: 'longsword-two-hands',
        revision: 2,
        expected: 'stale',
      })
    ).rejects.toMatchObject({ code: 'moved' });
    const left = (await handle('grip_library_remove_revision', {
      entry: 'longsword-two-hands',
      revision: 3,
      expected: read.version,
    })) as LibraryGripEntry;
    expect(left.revisions.map(r => r.revision)).toEqual([1, 2]);
    // A removed number is never handed out again.
    expect(left.lastRevision).toBe(3);
    const grip = (await get(handle, 'longsword-two-hands')).grip;
    const turned: GripFile = { ...grip, pose: { ...grip.pose, rightHand: [9, 0, -8] } };
    expect((await plan(handle, turned, 'longsword-two-hands')).revision).toBe(4);
    await expect(
      handle('grip_library_remove_revision', {
        entry: 'dagger-reverse',
        revision: 1,
        expected: null,
      })
    ).rejects.toMatchObject({ message: expect.stringMatching(/only revision/) });
    await expect(
      handle('grip_library_remove', { entry: 'dagger-reverse', expected: 'stale' })
    ).rejects.toMatchObject({ code: 'moved' });
    const went = await handle('grip_library_remove', { entry: 'dagger-reverse', expected: null });
    expect(went).toMatch(/^\.removed\/dagger-reverse-/);
    expect((await list(handle)).map(g => g.id)).not.toContain('dagger-reverse');
  });

  it('words what a revision changes as the core does', () => {
    const base: GripFile = {
      version: 1,
      name: 'Dagger',
      revision: 1,
      weapon: 'dagger',
      hands: { holds: 'right', supports: null },
      pose: { rightHand: [0, 30, 0], rightIndexProximal: [0, 0, 60] },
    };
    expect(gripChanges(null, base)).toEqual(['A new grip for right hand: 2 bones.']);
    expect(gripChanges(base, base)).toEqual([]);
    expect(
      gripChanges(base, {
        ...base,
        weapon: 'sword',
        hands: { holds: 'right', supports: 'left' },
        pose: { rightHand: [0, 35, 0], leftHand: [0, -30, 0], leftIndexProximal: [0, 0, -60] },
      })
    ).toEqual([
      'Holds sword instead of dagger.',
      'Poses both hands, where it posed right hand.',
      'Poses 2 bones more.',
      'Leaves 1 bone out.',
      'Turns 1 bone another way.',
    ]);
  });

  it('bakes the demo recipe: the grip on its own frames, every bone posed', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      const handle = backend();
      const bake = { project: 'frostveil', recipe: GRIP_DEMO, asset: 'clay-figure' };
      const started = (await handle('recipe_bake_start', { request: bake })) as RecipeBakeRecord;
      expect(started.state).toBe('running');
      await vi.advanceTimersByTimeAsync(10_000);
      const done = (await handle('recipe_bake_get', bake)) as RecipeBakeRecord;
      const frames = done.result!.frames;
      expect(done.result?.grips).toEqual([
        {
          layer: 'l-2',
          bone: 'rightHand',
          entry: 'scythe-staff-grip',
          revision: 1,
          frames: [Math.ceil(0.1 * frames), Math.floor(0.9 * frames)],
          bones: 33,
          posed: 33,
          missing: [],
        },
      ]);
      const line = done.review?.lines.find(l => l.id === 'grip-l-2');
      expect(line).toMatchObject({ label: 'Grip', value: '33 of 33 bones', status: 'pass' });
    } finally {
      vi.useRealTimers();
    }
  });
});
