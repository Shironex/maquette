import { describe, expect, it } from 'vitest';
import { FrameTimes, PIXEL_BUDGET, adaptiveDpr } from './quality';

describe('adaptiveDpr', () => {
  it('keeps the Canvas range as the ceiling', () => {
    expect(adaptiveDpr(1, { width: 1600, height: 900 })).toBe(1);
    expect(adaptiveDpr(2, { width: 1440, height: 900 })).toBe(1.5);
    expect(adaptiveDpr(0.8, { width: 1440, height: 900 })).toBe(1);
  });

  it('caps the ratio so the drawn pixels stay under the budget', () => {
    const size = { width: 3840, height: 2160 };
    const dpr = adaptiveDpr(1, size);
    expect(dpr).toBeLessThan(1);
    expect(size.width * size.height * dpr * dpr).toBeLessThanOrEqual(PIXEL_BUDGET * 1.01);
    // A retina laptop window fits at the full 1.5.
    expect(adaptiveDpr(2, { width: 1512, height: 860 })).toBe(1.5);
  });

  it('scales by the performance factor, never under the floor', () => {
    expect(adaptiveDpr(2, { width: 1440, height: 900 }, 0.6)).toBe(0.9);
    expect(adaptiveDpr(1, { width: 7680, height: 4320 }, 0.6)).toBe(0.5);
  });

  it('falls back to the ceiling before the canvas has a size', () => {
    expect(adaptiveDpr(2, { width: 0, height: 0 })).toBe(1.5);
  });
});

describe('FrameTimes', () => {
  const run = (times: FrameTimes, from: number, gaps: number[]) => {
    let now = from;
    for (const gap of gaps) times.tick((now += gap));
    return now;
  };

  it('needs enough frames before it answers', () => {
    const times = new FrameTimes();
    run(times, 0, Array(5).fill(30));
    expect(times.median()).toBeNull();
    expect(times.slow()).toBe(false);
  });

  it('skips the frame after a pause', () => {
    const times = new FrameTimes();
    let now = run(times, 0, Array(12).fill(16));
    // An idle loop, then a burst at 60 fps.
    now = run(times, now, [5000]);
    run(times, now, Array(3).fill(16));
    expect(times.median()).toBe(16);
    expect(times.slow()).toBe(false);
  });

  it('takes the median, so one slow frame does not count', () => {
    const times = new FrameTimes();
    run(times, 0, [...Array(12).fill(16), 90, 16, 16]);
    expect(times.slow()).toBe(false);
  });

  it('calls a GPU slow when its frames keep running long', () => {
    const times = new FrameTimes();
    run(times, 0, Array(15).fill(40));
    expect(times.slow()).toBe(true);
    // Every frame longer than a pause still counts after the first.
    const crawl = new FrameTimes();
    run(crawl, 0, Array(15).fill(300));
    expect(crawl.median()).toBe(300);
  });

  it('keeps the clock but not the gap of an uncounted frame', () => {
    const times = new FrameTimes();
    let now = run(times, 0, Array(12).fill(16));
    for (let i = 0; i < 20; i++) times.tick((now += 50), false);
    expect(times.median()).toBe(16);
  });
});
