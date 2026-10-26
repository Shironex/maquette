import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useTimelineStore } from '@/state/use-timeline-store';
import { clearMocks, openTimeline } from '@/test/open-timeline';
import { ExplodeLane } from '.';

afterEach(clearMocks);

describe('an explode lane', () => {
  it('shows the keys and the spread between them', async () => {
    await openTimeline();
    render(<ExplodeLane asset="switch-cabinet" />);
    expect(screen.getByRole('button', { name: 'Switch cabinet together at Canopy' })).toBeVisible();
    expect(screen.getByText('Spreads apart')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Switch cabinet apart at Pullback' }));
    expect(useTimelineStore.getState().selection).toEqual({
      kind: 'explode',
      asset: 'switch-cabinet',
      stop: 'pullback',
    });
  });

  it('keys a free stop', async () => {
    await openTimeline();
    render(<ExplodeLane asset="switch-cabinet" />);
    await userEvent.click(
      screen.getByRole('button', { name: 'Key Switch cabinet explode at Shrubs' })
    );
    const keys = useTimelineStore.getState().draft.timeline.explode['switch-cabinet']?.keys;
    expect(keys).toContainEqual({ stop: 'shrubs', amount: 1 });
  });

  it('draws nothing for a model without a track', async () => {
    await openTimeline();
    const { container } = render(<ExplodeLane asset="nothing" />);
    expect(container).toBeEmptyDOMElement();
  });
});
