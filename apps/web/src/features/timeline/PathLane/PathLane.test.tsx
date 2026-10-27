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
    render(<PathLane path="rune-line" />);
    expect(screen.getByRole('button', { name: 'Rune line dark at Veil gate' })).toBeVisible();
    expect(screen.getByText('Lights up')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Rune line lit at Keystone' }));
    expect(useTimelineStore.getState().selection).toEqual({
      kind: 'path',
      path: 'rune-line',
      stop: 'keystone',
    });
  });

  it('keys a free stop', async () => {
    await openTimeline();
    render(<PathLane path="rune-line" />);
    await userEvent.click(
      screen.getByRole('button', { name: 'Key Rune line energize at Arrival' })
    );
    const keys = useTimelineStore.getState().draft.timeline.paths['rune-line']?.keys;
    expect(keys).toContainEqual({ stop: 'arrival', amount: 1 });
  });

  it('draws nothing for a path without a track', async () => {
    await openTimeline();
    const { container } = render(<PathLane path="spare-run" />);
    expect(container).toBeEmptyDOMElement();
  });
});
