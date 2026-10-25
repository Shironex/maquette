import { describe, expect, it } from 'vitest';
import { STOP_MOTION, withMotion, type CameraStop } from './cameras';
import { STOP_IDS } from './stops';

const framing = (x: number): CameraStop => ({
  id: 'hero',
  position: [x, 1, 1],
  target: [x, 1, 0],
  fov: 30 + x,
});

describe('withMotion', () => {
  it('gives every site stop its framing in order with the site motion', () => {
    const stops = withMotion(STOP_IDS.map((_, i) => framing(i)));
    expect(stops.map(stop => stop.id)).toEqual([...STOP_IDS]);
    expect(stops[3]).toEqual({ ...framing(3), id: 'model', ...STOP_MOTION.model });
  });

  it('holds the last framing for stops the layout lacks', () => {
    const stops = withMotion([framing(0), framing(1)]);
    expect(stops).toHaveLength(STOP_IDS.length);
    expect(stops[6]?.position).toEqual([1, 1, 1]);
  });

  it('frames the whole bench when the layout has no stops', () => {
    const stops = withMotion([]);
    expect(new Set(stops.map(stop => stop.fov))).toEqual(new Set([45]));
  });
});
