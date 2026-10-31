import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { setExplodeKey } from '@/state/timeline';
import { useTimelineStore } from '@/state/use-timeline-store';
import { clearMocks, openTimeline } from '@/test/open-timeline';
import { TimelinePanel } from '.';

afterEach(clearMocks);

describe('the timeline panel', () => {
  it('asks for a pick and says what the site gets', async () => {
    await openTimeline();
    render(<TimelinePanel />);
    expect(screen.getByText(/^Pick a change or a key on the timeline/)).toBeVisible();
    expect(screen.getByText('Checks · 0')).toBeVisible();
    expect(screen.getByText('4 lanes, 7 marks')).toBeVisible();
  });

  it('warns about an explode track that never moves', async () => {
    await openTimeline();
    useTimelineStore.setState(s => ({
      draft: setExplodeKey(s.draft, 'switch-cabinet', 'pullback', 0),
    }));
    render(<TimelinePanel />);
    expect(screen.getByText('Switch cabinet never spreads')).toBeVisible();
  });

  it('fixes an explode track that never moves from its card', async () => {
    await openTimeline();
    useTimelineStore.setState(s => ({
      draft: setExplodeKey(s.draft, 'switch-cabinet', 'pullback', 0),
    }));
    render(<TimelinePanel />);
    await userEvent.click(screen.getByRole('button', { name: 'Spread by the last stop' }));
    expect(screen.queryByText('Switch cabinet never spreads')).toBeNull();
    expect(screen.getByText('Checks · 0')).toBeVisible();
  });

  it('ends a range that finishes off screen where the camera leaves', async () => {
    await openTimeline('looks');
    render(<TimelinePanel />);
    const checks = within(screen.getByRole('region', { name: 'Checks' }));
    expect(checks.getByText('Switch cabinet finishes off screen')).toBeVisible();
    await userEvent.click(checks.getByRole('button', { name: 'End at 50%' }));
    const range = useTimelineStore
      .getState()
      .draft.triggers['switch-cabinet']?.find(t => t.kind === 'scroll');
    expect(range?.end).toBe(50);
    expect(checks.queryByText('Switch cabinet finishes off screen')).toBeNull();
  });

  it('slides that range earlier, keeping its length', async () => {
    await openTimeline('looks');
    render(<TimelinePanel />);
    await userEvent.click(screen.getByRole('button', { name: 'Move it earlier' }));
    const range = useTimelineStore
      .getState()
      .draft.triggers['switch-cabinet']?.find(t => t.kind === 'scroll');
    expect(range).toMatchObject({ start: 25, end: 50 });
  });
});
