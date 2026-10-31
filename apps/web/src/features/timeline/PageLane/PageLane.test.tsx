import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useTimelineStore } from '@/state/use-timeline-store';
import { clearMocks, openTimeline } from '@/test/open-timeline';
import { PageLane } from '.';

afterEach(clearMocks);

describe('a page lane', () => {
  it('shows the render style and selects a key', async () => {
    await openTimeline('looks');
    render(<PageLane lane="style" />);
    expect(screen.getByText('Blueprint in')).toBeInTheDocument();
    await userEvent.click(
      screen.getByRole('button', { name: 'Render style blueprint at Keystone' })
    );
    expect(useTimelineStore.getState().selection).toEqual({
      kind: 'page',
      lane: 'style',
      stop: 'keystone',
    });
  });

  it('shows the hero text fading out and keys a free stop hidden', async () => {
    await openTimeline('looks');
    render(<PageLane lane="text" />);
    expect(screen.getByText('Fades out')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Key Hero text at Pullback' }));
    expect(useTimelineStore.getState().draft.timeline.text?.keys).toContainEqual({
      stop: 'pullback',
      amount: 0,
    });
  });

  it('draws nothing without a track', async () => {
    await openTimeline();
    const { container } = render(<PageLane lane="style" />);
    expect(container).toBeEmptyDOMElement();
  });
});
