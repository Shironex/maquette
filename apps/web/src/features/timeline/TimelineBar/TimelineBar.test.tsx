import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { usePageStore } from '@/state/use-page-store';
import { removeExplode } from '@/state/timeline';
import { useTimelineStore } from '@/state/use-timeline-store';
import { clearMocks, openTimeline } from '@/test/open-timeline';
import { TimelineBar } from '.';

const INITIAL = usePageStore.getState();

afterEach(() => {
  clearMocks();
  usePageStore.setState(INITIAL, true);
});

describe('the timeline toolbar', () => {
  it('says where the page is and plays it', async () => {
    await openTimeline();
    usePageStore.getState().seek(1.5);
    render(<TimelineBar />);
    expect(screen.getByRole('toolbar')).toHaveTextContent('38% · between Veil gate and Crystals');
    await userEvent.click(screen.getByRole('button', { name: 'Play the scroll at reading speed' }));
    expect(usePageStore.getState().playing).toBe(true);
    await userEvent.click(screen.getByRole('button', { name: 'Go to the top' }));
    expect(usePageStore.getState().target).toBe(0);
  });

  it('adds an explode track, and an energize track for a path without one', async () => {
    await openTimeline();
    // The cabinet is the one model with parts to spread, and it has its track already.
    act(() => useTimelineStore.getState().edit(d => removeExplode(d, 'switch-cabinet')));
    render(<TimelineBar />);
    await userEvent.click(screen.getByRole('button', { name: 'Add track' }));
    expect(screen.queryByRole('menuitem', { name: 'Energize Rune line' })).toBeNull();
    expect(screen.getByRole('menuitem', { name: 'New path...' })).toBeVisible();
    await userEvent.click(screen.getByRole('menuitem', { name: 'Explode Switch cabinet' }));
    expect(useTimelineStore.getState().selection).toMatchObject({
      kind: 'explode',
      asset: 'switch-cabinet',
    });
  });

  it('offers no explode track for a model that has one', async () => {
    await openTimeline();
    render(<TimelineBar />);
    await userEvent.click(screen.getByRole('button', { name: 'Add track' }));
    expect(screen.queryByRole('menuitem', { name: 'Explode Switch cabinet' })).toBeNull();
  });

  it('energizes a path from the menu', async () => {
    await openTimeline();
    render(<TimelineBar />);
    await userEvent.click(screen.getByRole('button', { name: 'Add track' }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Energize Spare run' }));
    const track = useTimelineStore.getState().draft.timeline.paths['spare-run'];
    expect(track?.keys.map(k => k.amount)).toEqual([0, 1]);
    expect(useTimelineStore.getState().selection).toEqual({
      kind: 'path',
      path: 'spare-run',
      stop: 'arrival',
    });
  });

  it('keeps Pixels off until the page answers with its scroll', async () => {
    await openTimeline();
    render(<TimelineBar />);
    expect(screen.getByRole('button', { name: 'Pixels' })).toHaveAttribute('aria-disabled', 'true');
    await userEvent.click(screen.getByRole('button', { name: 'Sections' }));
    expect(useTimelineStore.getState().units).toBe('stops');
  });
});
