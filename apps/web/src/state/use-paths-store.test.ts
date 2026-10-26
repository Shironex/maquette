import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { clearMocks, openPaths } from '@/test/open-paths';
import { movePoint } from './paths';
import { SAVE_DELAY, usePathsStore } from './use-paths-store';

afterEach(() => {
  vi.useRealTimers();
  clearMocks();
});

describe('the paths store', () => {
  it('reads the demo paths and their tracks and picks the first path', async () => {
    const s = await openPaths();
    expect(s.draft.paths.map(p => p.id)).toEqual(['lantern-cable', 'root-glow', 'spare-run']);
    expect(Object.keys(s.draft.tracks)).toEqual(['lantern-cable', 'root-glow']);
    expect(s.selection).toEqual({ path: 'lantern-cable', point: null });
  });

  it('saves an edit a moment later into the layout', async () => {
    await openPaths();
    vi.useFakeTimers();
    usePathsStore.getState().edit(d => movePoint(d, 'spare-run', 1, [2, 0.5, -2]), {
      path: 'spare-run',
      point: 1,
    });
    const saved = vi.spyOn(commands, 'pathsSave');
    await vi.advanceTimersByTimeAsync(SAVE_DELAY + 10);
    vi.useRealTimers();
    await vi.waitFor(() => expect(usePathsStore.getState().saving).toBe(false));
    expect(saved).toHaveBeenCalledOnce();
    const s = usePathsStore.getState();
    expect(s.draft).toEqual(s.saved);
    expect(s.view?.layout.paths[2]?.points[1]).toEqual([2, 0.5, -2]);
    expect(s.selection).toEqual({ path: 'spare-run', point: 1 });
  });

  it('reads a project without paths', async () => {
    const s = await openPaths('empty');
    expect(s.draft.paths).toEqual([]);
    expect(s.selection).toBeNull();
  });
});
