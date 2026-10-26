import { render, screen } from '@testing-library/react';
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
    expect(screen.getByText(/^Pick a change or an explode key/)).toBeVisible();
    expect(screen.getByText('Checks · 0')).toBeVisible();
    expect(screen.getByText('2 lanes, 4 marks')).toBeVisible();
  });

  it('warns about an explode track that never moves', async () => {
    await openTimeline();
    useTimelineStore.setState(s => ({
      draft: setExplodeKey(s.draft, 'switch-cabinet', 'pullback', 0),
    }));
    render(<TimelinePanel />);
    expect(screen.getByText('Switch cabinet never spreads')).toBeVisible();
  });
});
