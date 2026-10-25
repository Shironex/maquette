import { describe, expect, it } from 'vitest';
import type { LayoutModel } from '@/ipc/client';
import { sizeLine } from './size-line';

const crate = { min: [-0.25, 0, -0.1], max: [0.25, 1.5, 0.1] } as LayoutModel;

describe('a model size line', () => {
  it('gives width, height and depth in centimetres at the scale', () => {
    expect(sizeLine(crate, 1)).toBe('50.0 × 150 × 20.0 cm');
    expect(sizeLine(crate, 2)).toBe('100 × 300 × 40.0 cm');
  });

  it('says so when the bounds could not be read', () => {
    expect(sizeLine({ ...crate, min: null }, 1)).toBe('Size unknown');
  });
});
