import { describe, expect, it } from 'vitest';
import type { FirstView, LayoutModel, SceneLayout } from '@/ipc/client';
import {
  HISTORY,
  MAX_STOPS,
  addStop,
  dropToFloor,
  firstViewLine,
  fromLayout,
  layoutProblem,
  moveStop,
  nextStopName,
  normalDeg,
  pinAll,
  placedBounds,
  record,
  redo,
  removeStop,
  rotate,
  sizeCm,
  stopId,
  tidy,
  toLayout,
  undo,
  type PlainLayout,
  type PlainStop,
} from './layout';

function model(asset: string, over: Partial<LayoutModel> = {}): LayoutModel {
  return {
    asset,
    name: asset,
    kind: 'prop',
    version: 1,
    file: `/p/models/${asset}_v1.glb`,
    path: `models/${asset}_v1.glb`,
    min: [-0.5, 0, -0.25],
    max: [0.5, 2, 0.25],
    prep: { scale: 1, offset: [0, 0, 0] },
    fallback: [0, 0, 0],
    bytes: 1_000_000,
    measured: 'raw',
    ...over,
  };
}

function stop(id: string, over: Partial<PlainStop> = {}): PlainStop {
  return { id, name: id, position: [0, 1, 5], target: [0, 1, 0], fov: 35, models: null, ...over };
}

const empty: PlainLayout = { models: {}, stops: [], desk: null };
const close = (v: number[], want: number[]) => v.forEach((x, i) => expect(x).toBeCloseTo(want[i]!));

describe('the layout helpers', () => {
  it('reads and writes the saved shape without losing anything', () => {
    const saved: SceneLayout = {
      version: 1,
      models: {
        a: {
          position: [1, 0, 2],
          rotation: [0, 90, 0],
          scale: 2,
        },
      },
      stops: [
        {
          id: 'porch',
          name: 'Porch',
          position: [0, 1, 5],
          target: [0, 1, 0],
          fov: null,
          models: ['a'],
        },
      ],
      desk: { size: [1.2, 0.6] },
      states: {},
      clips: {},
    };
    const plain = fromLayout(saved);
    expect(plain.stops[0]!.fov).toBe(35);
    expect(plain.desk).toEqual([1.2, 0.6]);
    expect(toLayout(plain)).toEqual({ ...saved, stops: [{ ...saved.stops[0], fov: 35 }] });
  });

  it('measures a model in centimetres at its scale', () => {
    close(sizeCm(model('a'), 1.5), [150, 300, 75]);
  });

  it('turns points the way three.js does', () => {
    close(rotate([1, 0, 0], [0, 90, 0]), [0, 0, -1]);
    close(rotate([1, 0, 0], [0, 0, 90]), [0, 1, 0]);
    close(rotate([0, 1, 0], [90, 0, 0]), [0, 0, 1]);
  });

  it('places the bounds and drops a turned model onto the floor', () => {
    const lying = {
      position: [2, 3, 0] as [number, number, number],
      rotation: [0, 0, 90] as [number, number, number],
      scale: 1,
    };
    const { min, max } = placedBounds(model('a'), lying);
    close(min, [0, 2.5, -0.25]);
    close(max, [2, 3.5, 0.25]);
    expect(dropToFloor(model('a'), lying).position).toEqual([2, 0.5, 0]);
  });

  it('tidies a placement the way it is saved', () => {
    expect(tidy({ position: [0.123456, 0, 0], rotation: [0, 270, -540], scale: 0 })).toEqual({
      position: [0.1235, 0, 0],
      rotation: [0, -90, 180],
      scale: 0.01,
    });
    expect(normalDeg(-180)).toBe(180);
    expect(normalDeg(370)).toBe(10);
  });

  it('pins every model where it stands and keeps placements already set', () => {
    const layout = {
      ...empty,
      models: {
        a: {
          position: [1, 0, 0] as [number, number, number],
          rotation: [0, 0, 0] as [number, number, number],
          scale: 1,
        },
      },
    };
    const pinned = pinAll(layout, [model('a'), model('b', { fallback: [1.5, 0, 0] })]);
    expect(pinned.models.a!.position).toEqual([1, 0, 0]);
    expect(pinned.models.b).toEqual({ position: [1.5, 0, 0], rotation: [0, 0, 0], scale: 1 });
  });

  it('names, adds, orders and removes camera stops', () => {
    expect(stopId([stop('porch')], 'Porch')).toBe('porch-2');
    expect(stopId([], '  ')).toBe('stop');
    expect(nextStopName([stop('a', { name: 'Stop 2' })])).toBe('Stop 3');

    let layout = addStop(empty, { position: [1.23456, 2, 3], target: [0, 0, 0], fov: 40.123 });
    expect(layout.stops[0]).toMatchObject({
      id: 'stop-1',
      name: 'Stop 1',
      position: [1.2346, 2, 3],
      fov: 40.12,
    });
    layout = addStop(layout, { position: [0, 2, 3], target: [0, 0, 0], fov: 35 });
    layout = moveStop(layout, 'stop-2', -1);
    expect(layout.stops.map(s => s.id)).toEqual(['stop-2', 'stop-1']);
    expect(moveStop(layout, 'stop-2', -1)).toBe(layout);
    expect(removeStop(layout, 'stop-2').stops.map(s => s.id)).toEqual(['stop-1']);
  });

  it('says what keeps a layout from saving', () => {
    expect(layoutProblem({ ...empty, stops: [stop('a')] })).toBeNull();
    expect(layoutProblem({ ...empty, stops: [stop('a', { name: ' ' })] })).toMatch(/name/);
    expect(layoutProblem({ ...empty, stops: [stop('a', { target: [0, 1, 5] })] })).toMatch(
      /own position/
    );
    const many = Array.from({ length: MAX_STOPS + 1 }, (_, i) => stop(`s${i}`));
    expect(layoutProblem({ ...empty, stops: many })).toMatch(/24/);
  });

  it('undoes and redoes, and a new edit clears what could be redone', () => {
    const a = empty;
    const b = { ...empty, desk: [1, 1] as [number, number] };
    const c = { ...empty, desk: [2, 2] as [number, number] };
    let history = record({ past: [], future: [] }, a);
    const [h1, back] = undo(history, b)!;
    expect(back).toBe(a);
    const [h2, again] = redo(h1, a)!;
    expect(again).toBe(b);
    history = record(h1, a);
    expect(history.future).toEqual([]);
    expect(redo(history, c)).toBeNull();
    expect(undo({ past: [], future: [] }, a)).toBeNull();
    expect(h2.past).toEqual([a]);

    let long = { past: [], future: [] } as ReturnType<typeof record>;
    for (let i = 0; i < HISTORY + 5; i++) long = record(long, a);
    expect(long.past).toHaveLength(HISTORY);
  });

  it('says what the first view loads', () => {
    const view: FirstView = {
      stop: 'Porch',
      custom: false,
      models: [{ asset: 'a', name: 'A', bytes: 1_500_000, measured: 'raw' }],
      bytes: 1_500_000,
      targetMb: 3,
    };
    expect(firstViewLine(view)).toMatch(
      /^First view \(Porch\): 1 model in its frame, 1\.5 MB of 3 MB\.$/
    );
    expect(firstViewLine({ ...view, custom: true, models: [] })).toMatch(/0 models picked by hand/);
    expect(firstViewLine({ ...view, stop: null })).toMatch(/Add a camera stop/);
    expect(firstViewLine(null)).toMatch(/once the layout loads/);
  });
});
