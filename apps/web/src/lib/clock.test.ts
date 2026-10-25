import { afterEach, describe, expect, it, vi } from 'vitest';
import { clock } from './clock';

describe('clock', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it('reads the faked system time', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-01-02T03:04:05Z'));
    expect(clock.nowMs()).toBe(Date.UTC(2026, 0, 2, 3, 4, 5));
    expect(clock.now().toISOString()).toBe('2026-01-02T03:04:05.000Z');
  });
});
