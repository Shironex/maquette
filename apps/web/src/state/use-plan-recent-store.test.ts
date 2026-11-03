import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { events } from '@/ipc/generated/bindings';
import { clearMocks, openPlans } from '@/test/open-plan';
import { recentCanvases } from './plan-recent';
import { usePlanCameraStore } from './use-plan-camera-store';
import { RECENT_REREAD_MS, usePlanRecentStore } from './use-plan-recent-store';
import { usePlansStore } from './use-plans-store';

beforeEach(() => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  window.localStorage.clear();
});
afterEach(() => {
  usePlanRecentStore.getState().start(null, null);
  vi.useRealTimers();
  vi.restoreAllMocks();
  clearMocks();
});

const recent = () => usePlanRecentStore.getState();
const shown = () => recentCanvases(recent().list, recent().seen);

/** Frostveil's demo canvases, with the rail following the workspace's project. */
async function rail(root = '/mock/workspace') {
  const store = await openPlans();
  recent().start(root, 'frostveil');
  await vi.waitFor(() => expect(recent().list.length).toBeGreaterThan(0));
  // Planning events reach the rail as the shell wires them.
  const stop = await events.planChanged.listen(e =>
    recent().onChanged(e.payload.project, e.payload.id)
  );
  return { store, stop };
}

describe('the rail recent canvases', () => {
  it('opens a canvas from the rail and puts it first', async () => {
    await rail();
    await recent().open('opening-story');
    expect(usePlansStore.getState().open?.id).toBe('opening-story');
    expect(shown()[0]?.id).toBe('opening-story');
    expect(shown()[0]?.changed).toBe(false);
  });

  it('lists the camera plan once it is made and opened', async () => {
    await rail();
    expect(recent().list.some(p => p.id === 'camera-plan')).toBe(false);
    await usePlanCameraStore.getState().open('frostveil');
    vi.advanceTimersByTime(RECENT_REREAD_MS + 10);
    await vi.waitFor(() => expect(shown()[0]?.id).toBe('camera-plan'));
  });

  it('opens the camera plan with its panel', async () => {
    await rail();
    await usePlansStore.getState().loadList('frostveil');
    // The camera plan lists once it is made; the rail opens it as the camera plan.
    await usePlanCameraStore.getState().open('frostveil');
    await usePlansStore.getState().close();
    const open = vi.spyOn(usePlanCameraStore.getState(), 'open');
    await recent().open('camera-plan');
    expect(open).toHaveBeenCalledWith('frostveil');
  });

  it('marks a canvas an agent changed after the person left it, and follows renames and removals', async () => {
    const { store, stop } = await rail();
    await recent().open('combat-ideas');
    await usePlansStore.getState().close();
    vi.advanceTimersByTime(1000);
    await store.mock('mock_plan_agent', { project: 'frostveil', id: 'combat-ideas' });
    vi.advanceTimersByTime(RECENT_REREAD_MS + 10);
    await vi.waitFor(() => expect(shown().find(c => c.id === 'combat-ideas')?.changed).toBe(true));
    // Opening it again counts as seeing the change.
    await recent().open('combat-ideas');
    expect(shown().find(c => c.id === 'combat-ideas')?.changed).toBe(false);
    await usePlansStore.getState().close();

    // A rename here shows the new name and is no change to mark.
    expect(await usePlansStore.getState().rename('combat-ideas', 'Combat systems')).toBe(true);
    expect(shown().find(c => c.id === 'combat-ideas')).toMatchObject({
      name: 'Combat systems',
      changed: false,
    });
    // Removed to the trash: gone from the rail.
    expect(await usePlansStore.getState().remove('combat-ideas')).toBe(true);
    expect(recent().list.some(p => p.id === 'combat-ideas')).toBe(false);
    stop();
  });

  it('never marks the canvas open now, even when an agent changes it', async () => {
    const { store, stop } = await rail();
    await recent().open('combat-ideas');
    vi.advanceTimersByTime(1000);
    await store.mock('mock_plan_agent', { project: 'frostveil', id: 'combat-ideas' });
    vi.advanceTimersByTime(RECENT_REREAD_MS + 10);
    // The Plan view reads the open canvas again; the rail takes its time from there.
    const before = recent().list.find(p => p.id === 'combat-ideas')?.edited;
    await usePlansStore.getState().reload();
    await vi.waitFor(() =>
      expect(recent().list.find(p => p.id === 'combat-ideas')?.edited).not.toBe(before)
    );
    expect(shown().find(c => c.id === 'combat-ideas')?.changed).toBe(false);
    stop();
  });

  it('reads no list for the open canvas autosaving, and only the one canvas an agent changed', async () => {
    const { store, stop } = await rail();
    const list = vi.spyOn(commands, 'planList');
    const get = vi.spyOn(commands, 'planGet');
    await recent().open('combat-ideas');
    get.mockClear();
    for (let i = 0; i < 3; i += 1) {
      usePlansStore.getState().addNode('note', { x: 900 + i * 300, y: 900 });
      await usePlansStore.getState().save();
      vi.advanceTimersByTime(RECENT_REREAD_MS + 10);
    }
    await vi.waitFor(() =>
      expect(recent().list.find(p => p.id === 'combat-ideas')?.edited).toBe(
        usePlansStore.getState().open?.plan.edited
      )
    );
    expect(list).not.toHaveBeenCalled();
    expect(get).not.toHaveBeenCalled();
    // An agent's write to another canvas the rail knows reads that canvas alone.
    vi.advanceTimersByTime(1000);
    await store.mock('mock_plan_agent', { project: 'frostveil', id: 'boss-frost-warden' });
    vi.advanceTimersByTime(RECENT_REREAD_MS + 10);
    await vi.waitFor(() => expect(get).toHaveBeenCalledWith('frostveil', 'boss-frost-warden'));
    expect(list).not.toHaveBeenCalled();
    stop();
  });

  it('keeps each workspace and project to its own seen times', async () => {
    await rail('/mock/one');
    await recent().open('opening-story');
    expect(Object.keys(recent().seen)).toContain('opening-story');
    recent().start('/mock/two', 'frostveil');
    expect(recent().seen).toEqual({});
    recent().start('/mock/one', 'frostveil');
    expect(Object.keys(recent().seen)).toContain('opening-story');
    recent().start('/mock/one', null);
    expect(recent().list).toEqual([]);
  });
});
