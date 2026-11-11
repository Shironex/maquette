import { afterEach, describe, expect, it } from 'vitest';
import type { ReplaceReport, ShipSwap, SwapSide, UndoReport } from '@/ipc/client';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import {
  boundsWords,
  modelNumbers,
  replacedNotice,
  scaleWords,
  settingsOf,
  standingSize,
  swapLine,
  swapLines,
  toCheck,
  undoneNotice,
} from './built-replace';

const side = (over: Partial<SwapSide> = {}): SwapSide => ({
  version: 2,
  file: 'models/spiral_planner_v2.glb',
  shipsAs: 'spiral_planner_v2.glb',
  triangles: 94_600,
  bytes: 4_100_000,
  min: [-0.15, 0, -0.105],
  max: [0.15, 0.02, 0.105],
  scale: 0.3,
  ...over,
});

const report = (over: Partial<ReplaceReport> = {}): ReplaceReport => ({
  project: 'study-room',
  asset: 'spiral-planner',
  name: 'Spiral planner',
  built: 'planner-built',
  builtName: 'Planner built',
  script: 'built/planner.py',
  old: side(),
  new: side({ version: 3, file: 'models/spiral_planner_v3.glb', triangles: 1_170, scale: 1 }),
  bakes: true,
  parts: ['cover', 'spiral'],
  settings: [
    { what: 'Real size, 30 cm on its longest side', action: 'carried', why: 'A measure.' },
    { what: 'Decimate to 60k', action: 'reset', why: 'It repaired the old model.' },
    { what: 'Layout placement', action: 'kept', why: 'Keyed by the id.' },
  ],
  unresolved: [],
  warnings: [],
  note: 'Nothing is aligned.',
  trash: null,
  ...over,
});

const swap = (over: Partial<ShipSwap> = {}): ShipSwap => ({
  asset: 'spiral-planner',
  name: 'Spiral planner',
  script: 'built/planner.py',
  at: '2026-11-10T09:30:00.000Z',
  oldFile: 'spiral_planner_v2.glb',
  newFile: 'spiral_planner_v3.glb',
  oldVersion: 2,
  newVersion: 3,
  oldTriangles: 94_600,
  newTriangles: 1_170,
  oldBytes: 4_100_000,
  newBytes: 210_000,
  ...over,
});

afterEach(() => clearMocks());

describe('a built asset in the place of a model', () => {
  it('says how big each side stands, whether it is scaled and where its bounds are', () => {
    expect(standingSize(side())).toBe('30 × 2 × 21 cm');
    expect(standingSize(side({ min: [-1, 0, -0.5], max: [1, 2.5, 0.5] }))).toBe('2 × 2.5 × 1 m');
    expect(standingSize(side({ min: null }))).toBeNull();
    expect(scaleWords(side())).toBe('scaled by 0.3');
    expect(scaleWords(side({ scale: 0.99999994 }))).toBe('not scaled');
    expect(scaleWords(side({ scale: null }))).toBe('not scaled');
    expect(boundsWords(side())).toBe('-0.15, 0, -0.1 to 0.15, 0.02, 0.1');
    expect(boundsWords(side({ max: null }))).toBeNull();
    expect(modelNumbers(side())).toBe('94.6k triangles · 4.1 MB');
    expect(modelNumbers(side({ triangles: null, bytes: null }))).toBe('');
  });

  it('groups the settings by what happens to them', () => {
    const found = report();
    expect(settingsOf(found, 'carried').map(s => s.what)).toEqual([
      'Real size, 30 cm on its longest side',
    ]);
    expect(settingsOf(found, 'brought')).toEqual([]);
    expect(toCheck(report({ warnings: ['a'], unresolved: ['b', 'c'] }))).toBe(3);
  });

  it('tells a clean replace from one with things to check', () => {
    expect(replacedNotice(report())).toEqual({
      status: 'done',
      title: "Planner built is in Spiral planner's place",
      detail: 'v3 is the built model. Bring back the old model undoes it.',
    });
    const check = replacedNotice(report({ warnings: ['The built model sits apart.'] }));
    expect(check.status).toBe('info');
    expect(check.detail).toBe(
      'v3 is the built model, with 1 thing to check. Bring back the old model undoes it.'
    );
  });

  it('says what came back after an undo', () => {
    const undone: UndoReport = {
      project: 'study-room',
      asset: 'spiral-planner',
      name: 'Spiral planner',
      script: 'built/planner.py',
      back: { version: 2, file: 'models/spiral_planner_v2.glb', triangles: 94_600, bytes: 1 },
      takenOff: [],
      notBuilt: [],
      setAside: null,
      restores: 'Planner built',
      settings: [],
      warnings: [],
    };
    expect(undoneNotice(undone)).toEqual({
      status: 'done',
      title: 'Spiral planner is back on v2',
      detail: 'Planner built is back on the Board as a built asset.',
    });
    expect(undoneNotice({ ...undone, restores: null }).detail).toBe(
      'The model it had before is current again.'
    );
    expect(undoneNotice({ ...undone, warnings: ['Its card did not come back.'] })).toMatchObject({
      status: 'info',
      detail: 'Its card did not come back.',
    });
  });

  it('writes the line the Ship approval shows for a swap', () => {
    expect(swapLine(swap())).toBe(
      'Spiral planner is a built model now: spiral_planner_v3.glb takes the place of spiral_planner_v2.glb · 1.2k triangles, was 94.6k · 210 KB, was 4.1 MB'
    );
    // A hand-built site and a game keep one file name.
    expect(swapLine(swap({ oldFile: 'planner.glb', newFile: 'planner.glb' }))).toContain(
      'is a built model now: planner.glb · 1.2k triangles'
    );
    expect(swapLine(swap({ oldFile: null, newFile: null, oldBytes: null }))).toBe(
      'Spiral planner is a built model now: v3 takes the place of v2 · 1.2k triangles, was 94.6k'
    );
    // An undo after a Ship: the old model takes the built one's place again.
    const back = swap({
      undone: true,
      oldFile: 'spiral_planner_v3.glb',
      newFile: 'spiral_planner_v2.glb',
    });
    expect(swapLine(back)).toMatch(
      /^Spiral planner goes back to its old model: spiral_planner_v2\.glb takes the place of spiral_planner_v3\.glb/
    );
  });

  it('reads the swaps that wait for a Ship, and none when there are none', async () => {
    await openMockWorkspace('configured', undefined, undefined, undefined, undefined, 'swapped');
    const lines = await swapLines('frostveil');
    expect(lines).toHaveLength(1);
    expect(lines[0]).toContain('Crystal lantern is a built model now');
    expect(lines[0]).toContain('crystal_lantern_v3.glb takes the place of crystal_lantern_v2.glb');
    clearMocks();
    await openMockWorkspace('configured', undefined, undefined, undefined, undefined, 'swap');
    expect(await swapLines('frostveil')).toEqual([]);
  });
});
