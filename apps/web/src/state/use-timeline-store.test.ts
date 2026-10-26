import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { clearMocks, openTimeline } from '@/test/open-timeline';
import { addStopMark } from './timeline';
import { SAVE_DELAY, useTimelineStore } from './use-timeline-store';

afterEach(() => {
  vi.useRealTimers();
  clearMocks();
});

describe('the timeline store', () => {
  it('reads the stops, the cabinet lane and its explode track', async () => {
    const s = await openTimeline();
    expect(s.view?.stops.map(x => x.id)).toEqual([
      'porch',
      'sakura',
      'shrubs',
      'canopy',
      'pullback',
    ]);
    expect(s.draft.triggers['switch-cabinet']?.map(t => t.kind)).toEqual(['stop', 'scroll']);
    expect(s.draft.timeline.explode['switch-cabinet']?.keys).toHaveLength(2);
  });

  it('saves an edit a moment later and keeps the click trigger', async () => {
    await openTimeline();
    vi.useFakeTimers();
    const { view } = useTimelineStore.getState();
    const object = view!.states['switch-cabinet']!;
    useTimelineStore
      .getState()
      .edit(d => addStopMark(d, 'switch-cabinet', object, view!.stops, 'canopy')!.draft, {
        kind: 'state',
        asset: 'switch-cabinet',
        index: 2,
      });
    expect(useTimelineStore.getState().selection).toEqual({
      kind: 'state',
      asset: 'switch-cabinet',
      index: 2,
    });
    const saved = vi.spyOn(commands, 'timelineSave');
    await vi.advanceTimersByTimeAsync(SAVE_DELAY + 10);
    vi.useRealTimers();
    await vi.waitFor(() => expect(useTimelineStore.getState().saving).toBe(false));
    expect(saved).toHaveBeenCalledOnce();
    const s = useTimelineStore.getState();
    expect(s.draft).toEqual(s.saved);
    expect(s.view?.states['switch-cabinet']?.triggers.map(t => t.kind)).toEqual([
      'click',
      'stop',
      'scroll',
      'stop',
    ]);
  });

  it('reads a project without camera stops', async () => {
    const s = await openTimeline('empty');
    expect(s.view?.stops).toEqual([]);
  });
});
