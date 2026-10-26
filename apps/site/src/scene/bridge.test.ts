import { describe, expect, it } from 'vitest';
import { frameStats, readMessage } from './bridge';

describe('readMessage', () => {
  it('reads hello and goto from Maquette', () => {
    expect(readMessage({ maquette: 1, type: 'hello' })).toEqual({ maquette: 1, type: 'hello' });
    expect(readMessage({ maquette: 1, type: 'goto', stop: 2.5 })).toEqual({
      maquette: 1,
      type: 'goto',
      stop: 2.5,
      smooth: false,
    });
    expect(readMessage({ maquette: 1, type: 'goto', stop: 1, smooth: true })?.type).toBe('goto');
  });

  it('ignores anything else on the channel', () => {
    for (const data of [
      null,
      'hello',
      { type: 'hello' },
      { maquette: 2, type: 'hello' },
      { maquette: 1, type: 'goto' },
      { maquette: 1, type: 'goto', stop: Number.NaN },
      { maquette: 1, type: 'reload' },
    ]) {
      expect(readMessage(data)).toBeNull();
    }
  });
});

describe('frameStats', () => {
  it('gives the mean and the 95th percentile of frame intervals', () => {
    const deltas = [
      ...Array.from({ length: 95 }, () => 16),
      ...Array.from({ length: 5 }, () => 40),
    ];
    expect(frameStats(deltas)).toEqual({ frameMs: 17.2, p95Ms: 40 });
    expect(frameStats([10])).toEqual({ frameMs: 10, p95Ms: 10 });
  });

  it('has nothing to say before the first frame', () => {
    expect(frameStats([])).toBeNull();
  });
});
