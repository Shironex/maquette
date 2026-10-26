import { describe, expect, it } from 'vitest';
import {
  HEAD,
  SOFT,
  energizeAt,
  headAt,
  liftPaths,
  litAt,
  polylineLength,
  tubeSegments,
  type ScenePath,
} from './paths';
import type { PathTrack } from './timeline';

const STOPS = ['porch', 'sakura', 'canopy'];

const feed = (
  points: ScenePath['points'] = [
    [0, 0, 0],
    [3, 0, 0],
    [3, 4, 0],
  ]
): ScenePath => ({
  id: 'main-feed',
  name: 'Main feed',
  curve: 'polyline',
  points,
  start: null,
  end: null,
  width: 0.04,
  color: '#f2c230',
  glow: '#ffe08a',
});

const track = (easing: PathTrack['easing'] = 'linear'): PathTrack => ({
  easing,
  keys: [
    { stop: 'canopy', amount: 1 },
    { stop: 'sakura', amount: 0 },
  ],
});

describe('the path fill', () => {
  it('is dark at 0 and lit end to end at 1', () => {
    for (const u of [0, 0.3, 1]) {
      expect(litAt(u, 0)).toBe(0);
      expect(litAt(u, 1)).toBe(1);
    }
  });

  it('lights the path behind the front and fades ahead of it', () => {
    expect(litAt(0.1, 0.5)).toBe(1);
    expect(litAt(0.9, 0.5)).toBe(0);
    const front = 0.5 * (1 + SOFT);
    expect(litAt(front - SOFT / 2, 0.5)).toBeCloseTo(0.5);
    expect(litAt(front, 0.5)).toBe(0);
  });

  it('rides a head on the front only while it fills', () => {
    const front = 0.5 * (1 + SOFT) - SOFT;
    expect(headAt(front, 0.5)).toBe(1);
    expect(headAt(front + HEAD / 2, 0.5)).toBeCloseTo(0.5);
    expect(headAt(front + HEAD * 2, 0.5)).toBe(0);
    expect(headAt(0, 0)).toBe(0);
    expect(headAt(1, 1)).toBe(0);
  });

  it('eases the amount between the keyed stops and holds it outside them', () => {
    expect(energizeAt(track(), STOPS, 0)).toBe(0);
    expect(energizeAt(track(), STOPS, 1)).toBe(0);
    expect(energizeAt(track(), STOPS, 1.5)).toBeCloseTo(0.5);
    expect(energizeAt(track('ease-in'), STOPS, 1.5)).toBeCloseTo(0.125);
    expect(energizeAt(track(), STOPS, 2.4)).toBe(1);
  });

  it('stays dark without a track or with keys at stops the site does not have', () => {
    expect(energizeAt(undefined, STOPS, 1)).toBe(0);
    const lost: PathTrack = { easing: 'linear', keys: [{ stop: 'gone', amount: 1 }] };
    expect(energizeAt(lost, STOPS, 1)).toBe(0);
  });
});

describe('the path tube', () => {
  it('measures the path and rings it about every two widths', () => {
    expect(polylineLength(feed().points)).toBeCloseTo(7);
    expect(tubeSegments(feed())).toBe(88);
    expect(
      tubeSegments(
        feed([
          [0, 0, 0],
          [0.01, 0, 0],
        ])
      )
    ).toBe(16);
    expect(
      tubeSegments({
        ...feed(),
        width: 0.001,
        points: [
          [0, 0, 0],
          [50, 0, 0],
        ],
      })
    ).toBe(256);
  });

  it('lifts the points onto the bench and skips a path without two points', () => {
    const [lifted, ...rest] = liftPaths([feed(), feed([[0, 0, 0]])], 0.9);
    expect(rest).toHaveLength(0);
    expect(lifted?.points[2]).toEqual([3, 4.9, 0]);
    expect(liftPaths(undefined, 0.9)).toEqual([]);
  });
});
