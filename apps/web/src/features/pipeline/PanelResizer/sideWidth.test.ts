import { describe, expect, it } from 'vitest';
import { SIDE_WIDTH, clampWidth, keyWidth } from './sideWidth';

describe('the side panel width', () => {
  it('stays inside its limits and leaves the log room', () => {
    expect(clampWidth(100)).toBe(SIDE_WIDTH.min);
    expect(clampWidth(5000)).toBe(SIDE_WIDTH.max);
    expect(clampWidth(700, 900)).toBe(540);
    expect(clampWidth(700, 500)).toBe(SIDE_WIDTH.min);
  });

  it('moves with the arrow keys, Home and End', () => {
    expect(keyWidth('ArrowLeft', 500)).toBe(524);
    expect(keyWidth('ArrowRight', 500)).toBe(476);
    expect(keyWidth('Home', 500)).toBe(SIDE_WIDTH.max);
    expect(keyWidth('End', 500)).toBe(SIDE_WIDTH.min);
    expect(keyWidth('a', 500)).toBeNull();
  });
});
