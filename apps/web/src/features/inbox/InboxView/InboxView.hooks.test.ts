import { renderHook } from '@testing-library/react';
import { act } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { readLastRun } from '@/lib/waiting';
import { useInboxStore } from '@/state/use-inbox-store';
import { useInboxView } from './InboxView.hooks';

describe('the inbox view wait', () => {
  beforeEach(() => {
    window.localStorage.clear();
    vi.useFakeTimers();
    vi.setSystemTime(1_000_000);
    useInboxStore.setState({ snapshot: null, loading: true, loadFailed: false });
  });
  afterEach(() => {
    vi.useRealTimers();
    useInboxStore.setState({ snapshot: null, loading: false, loadFailed: false });
  });

  it('does not save a failed read as a past run', () => {
    renderHook(() => useInboxView());
    vi.setSystemTime(1_030_000);
    act(() => useInboxStore.setState({ loading: false, loadFailed: true }));
    expect(readLastRun('inbox')).toBeNull();
  });

  it('saves a read that came back', () => {
    renderHook(() => useInboxView());
    vi.setSystemTime(1_030_000);
    act(() => useInboxStore.setState({ loading: false, snapshot: { items: [] } as never }));
    expect(readLastRun('inbox')).toBe(30_000);
  });
});
