import { describe, expect, it } from 'vitest';
import { extentOf, formatDims, formatLength, frameOf, framed, scaleFor } from './real-size';

describe('real size maths', () => {
  it('scales a model so the named side measures the size, as the core does', () => {
    const extent = extentOf([-0.25, 0, -0.25], [0.25, 0.7, 0.25]);
    expect(scaleFor({ axis: 'height', value: 45, unit: 'cm' }, extent)).toBeCloseTo(0.6429, 4);
    expect(scaleFor({ axis: 'width', value: 1, unit: 'm' }, extent)).toBeCloseTo(2);
    expect(scaleFor({ axis: 'longest', value: 35, unit: 'cm' }, extent)).toBeCloseTo(0.5);
    expect(scaleFor({ axis: 'height', value: 1, unit: 'm' }, [1, 0, 1])).toBeNull();
  });

  it('moves the bottom centre to the origin before it scales', () => {
    const frame = frameOf(
      { size: { axis: 'height', value: 45, unit: 'cm' }, fixups: [{ op: 'origin-bottom' }] },
      [0.05, 0.4975, -0.45],
      [0.55, 1.1975, 0.05]
    );
    expect(frame.offset[0]).toBeCloseTo(-0.3);
    expect(frame.offset[1]).toBeCloseTo(-0.4975);
    expect(frame.offset[2]).toBeCloseTo(0.2);
    const top = framed(frame, [0.55, 1.1975, 0.05]);
    expect(top[1]).toBeCloseTo(0.45);
    expect(frameOf(null, [0, 0, 0], [1, 1, 1])).toEqual({ scale: 1, offset: [0, 0, 0] });
  });

  it('writes lengths in the unit that reads best', () => {
    expect(formatLength(0.45)).toBe('45 cm');
    expect(formatLength(1.2)).toBe('1.2 m');
    expect(formatLength(0.008)).toBe('8 mm');
    expect(formatDims([0.62, 1.58, 0.31])).toBe('0.62 × 1.58 × 0.31 m');
    expect(formatDims([0.5, 0.7, 0.5])).toBe('50 × 70 × 50 cm');
  });
});
