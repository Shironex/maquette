import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useTimelineStore } from '@/state/use-timeline-store';
import { clearMocks, openTimeline } from '@/test/open-timeline';
import { PathLane } from '.';

afterEach(clearMocks);

describe('a path lane', () => {
  it('shows the keys and the fill between them', async () => {
    await openTimeline();
    render(<PathLane path="lantern-cable" />);
    expect(screen.getByRole('button', { name: 'Lantern cable dark at Sakura' })).toBeVisible();
    expect(screen.getByText('Lights up')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Lantern cable lit at Canopy' }));
    expect(useTimelineStore.getState().selection).toEqual({
      kind: 'path',
      path: 'lantern-cable',
      stop: 'canopy',
    });
  });

  it('keys a free stop', async () => {
    await openTimeline();
    render(<PathLane path="lantern-cable" />);
    await userEvent.click(
      screen.getByRole('button', { name: 'Key Lantern cable energize at Porch' })
    );
    const keys = useTimelineStore.getState().draft.timeline.paths['lantern-cable']?.keys;
    expect(keys).toContainEqual({ stop: 'porch', amount: 1 });
  });

  it('draws nothing for a path without a track', async () => {
    await openTimeline();
    const { container } = render(<PathLane path="spare-run" />);
    expect(container).toBeEmptyDOMElement();
  });
});
