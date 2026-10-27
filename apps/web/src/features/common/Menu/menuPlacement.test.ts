import { describe, expect, it } from 'vitest';
import { placeMenu } from './menuPlacement';

const view = { width: 1000, height: 700 };
const box = (left: number, top: number, width: number, height: number) => ({
  left,
  top,
  right: left + width,
  bottom: top + height,
});

describe('placeMenu', () => {
  it('keeps a menu that fits where its caller put it', () => {
    const place = placeMenu(box(100, 120, 180, 200), box(100, 90, 30, 26), view);
    expect(place).toMatchObject({ left: 100, top: 120, width: 180 });
  });

  it('shifts a menu back inside the left and right edges', () => {
    expect(placeMenu(box(-30, 120, 180, 200), box(0, 90, 30, 26), view).left).toBe(8);
    expect(placeMenu(box(900, 120, 180, 200), box(900, 90, 30, 26), view).left).toBe(812);
  });

  it('flips above the anchor when there is more room above than below', () => {
    const place = placeMenu(box(100, 560, 180, 300), box(100, 530, 30, 26), view);
    expect(place.top + place.maxHeight).toBe(526);
    expect(place.top).toBeGreaterThanOrEqual(8);
  });

  it('caps a menu taller than the room and keeps it inside the window', () => {
    const place = placeMenu(box(100, 40, 180, 2000), box(100, 10, 30, 26), view);
    expect(place.maxHeight).toBe(652);
    expect(place.top + place.maxHeight).toBeLessThanOrEqual(692);
  });
});
