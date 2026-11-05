import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type { LayoutView } from '@/ipc/client';
import { layoutSpot, placementMatrix } from './recipe-layout';
import { targetsOf } from './recipe-pose';

/** The core's placement cases (`layout/placement_cases.json`), which its own tests run too. */
const placements = JSON.parse(
  readFileSync(
    resolve(
      import.meta.dirname,
      '../../../../../crates/maquette-model/src/layout/placement_cases.json'
    ),
    'utf8'
  )
) as {
  cases: {
    position: [number, number, number];
    rotation: [number, number, number];
    scale: number;
    matrix: number[];
  }[];
};

/** Elsheria turned a quarter about Y, a lantern 1 m along world +Z. */
function view(placeModel = true, present = true): LayoutView {
  const models: LayoutView['layout']['models'] = {
    lantern: { position: [0, 0, 1], rotation: [0, 0, 0], scale: 1 },
  };
  if (placeModel) models.elsheria = { position: [0, 0, 0], rotation: [0, 90, 0], scale: 1 };
  return {
    project: 'frostveil',
    saved: true,
    layout: { models, stops: [], desk: null } as unknown as LayoutView['layout'],
    models: [
      { asset: 'lantern', present, min: [-0.1, 0, -0.1], max: [0.1, 0.4, 0.1] },
    ] as unknown as LayoutView['models'],
    firstView: {} as LayoutView['firstView'],
  };
}

describe('a prop target by the layout', () => {
  it("sits at the prop's middle seen from the model, as the bake aims", () => {
    const spot = layoutSpot(view(), 'elsheria', 'lantern')!;
    // World (0, 0.2, 1) in a frame turned 90 degrees about Y is (-1, 0.2, 0).
    expect(spot.map(n => Number(n.toFixed(6)))).toEqual([-1, 0.2, 0]);
  });

  it('is none unless the layout places both, so the slot is used', () => {
    expect(layoutSpot(view(false), 'elsheria', 'lantern')).toBeNull();
    expect(layoutSpot(view(), 'elsheria', 'cup')).toBeNull();
    // As in the bake, a prop whose newest model file is gone takes the slot.
    expect(layoutSpot(view(true, false), 'elsheria', 'lantern')).toBeNull();
  });

  it('composes a placement as the core does, by the shared cases', () => {
    for (const c of placements.cases) {
      const m = placementMatrix({
        position: c.position,
        rotation: c.rotation,
        scale: c.scale,
      });
      m.elements.forEach((got, i) => expect(got).toBeCloseTo(c.matrix[i]!, 5));
    }
  });

  it('puts a placed prop at its spot and keeps the next prop in the second slot', () => {
    const at = [-1, 0.2, 0] as const;
    const targets = targetsOf(
      [
        { id: 'lantern', kind: 'prop', point: null, at },
        { id: 'cup', kind: 'prop', point: null },
      ],
      2
    );
    expect(targets.get('lantern')!.toArray()).toEqual([-1, 0.2, 0]);
    // The second slot sits on the other side from the first.
    expect(targets.get('cup')!.x).toBeCloseTo(0.44, 6);
  });
});
