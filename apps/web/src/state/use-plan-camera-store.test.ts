import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { clock } from '@/lib/clock';
import { clearMocks, openPlans } from '@/test/open-plan';
import { CAMERA_PLAN_ID, moved } from './plan-camera';
import { usePlanCameraStore } from './use-plan-camera-store';
import { usePlanLogStore } from './use-plan-log-store';
import { usePlansStore } from './use-plans-store';

beforeEach(() => vi.useFakeTimers({ shouldAdvanceTime: true }));
afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  clearMocks();
});

const camera = () => usePlanCameraStore.getState();
const plans = () => usePlansStore.getState();
const layoutIds = async () => (await commands.layoutGet('frostveil')).layout.stops.map(s => s.id);

/** Opens Frostveil's camera plan, made from its five layout stops. */
async function openCamera() {
  const store = await openPlans();
  expect(await camera().open('frostveil')).toBe(true);
  await camera().refresh('frostveil');
  return store;
}

/** Moves the second stop of the main line first and saves it. */
async function swapFirstTwo() {
  const main = camera().view!.main;
  const next = moved(plans().nodes, plans().edges, main, main[1]!, -1, clock.nowMs());
  plans().replaceFlow(next!.nodes, next!.edges);
  expect(await plans().save()).toBe(true);
  await camera().refresh('frostveil');
}

describe('the camera plan store', () => {
  it('opens a plan built from layout.json, its stops joined in tour order', async () => {
    await openCamera();
    expect(plans().open?.id).toBe(CAMERA_PLAN_ID);
    expect(plans().nodes.filter(n => n.data.node.kind === 'stop')).toHaveLength(5);
    expect(plans().edges.every(e => e.data?.edge.kind === 'leads-to')).toBe(true);
    expect(camera().view?.main).toHaveLength(5);
    expect(camera().view?.stale).toBe(false);
    // Nothing differs yet, so Apply has nothing to write.
    expect(camera().review?.problems[0]).toMatch(/^Nothing to apply/);
  });

  it('applies a reviewed order through the review, showing the moved stop, and logs it', async () => {
    await openCamera();
    await swapFirstTwo();
    await camera().askApply();
    const review = camera().apply!.review;
    expect(review.diff.moved).toBe(1);
    expect(review.diff.lines.some(l => l.note === 'moves to 2' || l.note === 'moved from 2')).toBe(
      true
    );
    expect(await camera().confirmApply()).toBe(true);
    expect(await layoutIds()).toEqual(['veil-gate', 'arrival', 'crystals', 'keystone', 'pullback']);
    const lines = usePlanLogStore.getState().lines;
    expect(lines[0]?.mark).toBe('apply');
    expect(camera().view?.stale).toBe(false);
  });

  it('refuses a review shown before layout.json changed, and reviews again', async () => {
    const store = await openCamera();
    await swapFirstTwo();
    await camera().askApply();
    await store.mock('mock_layout_outside', { project: 'frostveil' });
    expect(await camera().confirmApply()).toBe(false);
    expect(camera().apply?.error).toMatch(/changed since this review was shown/);
    expect(await layoutIds()).toEqual(['veil-gate', 'arrival', 'crystals', 'keystone', 'pullback']);
    await camera().reviewAgain();
    expect(camera().apply?.error).toBeNull();
    expect(camera().apply?.review.stale).toBe(true);
  });

  it('undoes an Apply while layout.json is as it left it, and not after it changed', async () => {
    const store = await openCamera();
    const before = await layoutIds();
    await swapFirstTwo();
    await camera().askApply();
    await camera().confirmApply();
    const applied = usePlanLogStore.getState().lines[0]!.id;
    expect(await camera().undoApply(applied)).toBe(true);
    expect(await layoutIds()).toEqual(before);

    // The plan still holds the new order: Apply it again, then change layout.json under it.
    await camera().askApply();
    expect(await camera().confirmApply()).toBe(true);
    const again = usePlanLogStore.getState().lines.find(l => l.mark === 'apply' && !l.undone)!;
    await store.mock('mock_layout_outside', { project: 'frostveil' });
    expect(await camera().undoApply(again.id)).toBe(false);
  });

  it('shows the plan behind layout.json and rebases it with the outside change', async () => {
    const store = await openCamera();
    await store.mock('mock_layout_outside', { project: 'frostveil' });
    await camera().refresh('frostveil');
    expect(camera().view?.stale).toBe(true);
    expect(camera().view?.outside.moved).toBeGreaterThan(0);
    await camera().askCompare();
    expect(camera().compare?.report.taken).toContain('Keystone: fov');
    expect(await camera().rebase('rebase')).toBe(true);
    expect(camera().view?.stale).toBe(false);
    expect(camera().compare).toBeNull();
  });
});
