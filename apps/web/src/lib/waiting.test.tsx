import { renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { lastRunLabel, readLastRun, rememberRun, useWait, waitCount } from './waiting';

describe('the waiting recipe', () => {
  beforeEach(() => {
    window.localStorage.clear();
    vi.useFakeTimers();
    vi.setSystemTime(1_000_000);
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it('counts and says a past time only when there is one worth saying', () => {
    expect(waitCount(3, 13)).toBe('3 of 13');
    expect(lastRunLabel(null)).toBeNull();
    expect(lastRunLabel(800)).toBeNull();
    expect(lastRunLabel(12_000)).toBe('last 12s');
    expect(lastRunLabel(64_000)).toBe('last 1m 04s');
  });

  it('says nothing on the first wait, and what it measured on the next', () => {
    const { result, rerender } = renderHook(({ active }) => useWait('motion:frostveil', active), {
      initialProps: { active: true },
    });
    expect(result.current).toBeNull();
    vi.setSystemTime(1_012_000);
    rerender({ active: false });
    expect(readLastRun('motion:frostveil')).toBe(12_000);
    rerender({ active: true });
    expect(result.current).toBe('last 12s');
  });

  it('does not remember a wait that failed', () => {
    rememberRun('x', 5000);
    const { rerender } = renderHook(({ active, failed }) => useWait('x', active, failed), {
      initialProps: { active: true, failed: false },
    });
    vi.setSystemTime(1_090_000);
    rerender({ active: false, failed: true });
    expect(readLastRun('x')).toBe(5000);
  });

  it('does not remember a wait left behind', () => {
    const { unmount } = renderHook(() => useWait('left', true));
    vi.setSystemTime(1_090_000);
    unmount();
    expect(readLastRun('left')).toBeNull();
  });

  it('leaves a real time be when a wait was too short to mention', () => {
    rememberRun('cached', 12_000);
    rememberRun('cached', 200);
    expect(readLastRun('cached')).toBe(12_000);
    rememberRun('cached', 2000);
    expect(readLastRun('cached')).toBe(2000);
  });

  it('times only the newest question of a wait that never stopped', () => {
    const { rerender } = renderHook(({ active, ask }) => useWait('p', active, false, ask), {
      initialProps: { active: true, ask: 1 },
    });
    vi.setSystemTime(1_030_000);
    rerender({ active: true, ask: 2 });
    vi.setSystemTime(1_035_000);
    rerender({ active: false, ask: 2 });
    expect(readLastRun('p')).toBe(5000);
  });

  it('starts over when the wait changes name', () => {
    const { rerender } = renderHook(({ key }) => useWait(key, true), {
      initialProps: { key: 'a' },
    });
    vi.setSystemTime(1_090_000);
    rerender({ key: 'b' });
    expect(readLastRun('a')).toBeNull();
  });
});
