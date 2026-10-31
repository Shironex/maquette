import { describe, expect, it } from 'vitest';
import { lookFilter, styleAt, textAt, type StyleTrack, type TextTrack } from './page-tracks';

const stops = ['a', 'b', 'c'];
const style: StyleTrack = {
  look: 'blueprint',
  easing: 'linear',
  keys: [
    { stop: 'a', amount: 0 },
    { stop: 'b', amount: 1 },
  ],
};
const text: TextTrack = {
  easing: 'linear',
  keys: [
    { stop: 'a', amount: 1 },
    { stop: 'b', amount: 0 },
  ],
};

describe('the page tracks', () => {
  it('leaves the scene as built at 0 and fades into the look', () => {
    expect(lookFilter('clay', 0)).toBe('none');
    expect(lookFilter('blueprint', 1)).toContain('hue-rotate(170deg)');
    expect(styleAt(style, stops, 0)).toBe('none');
    expect(styleAt(style, stops, 0.5)).toContain('hue-rotate(85deg)');
    expect(styleAt(style, stops, 2)).toContain('hue-rotate(170deg)');
  });

  it('draws nothing without a track or keys', () => {
    expect(styleAt(null, stops, 1)).toBe('none');
    expect(styleAt({ ...style, keys: [] }, stops, 1)).toBe('none');
  });

  it('fades the hero copy and shows it without a track', () => {
    expect(textAt(text, stops, 0)).toBe(1);
    expect(textAt(text, stops, 0.5)).toBeCloseTo(0.5);
    expect(textAt(text, stops, 1.5)).toBe(0);
    expect(textAt(undefined, stops, 2)).toBe(1);
  });
});
