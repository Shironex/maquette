import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useTimelineStore } from '@/state/use-timeline-store';
import { clearMocks, openTimeline } from '@/test/open-timeline';
import { ExplodeKeyEditor } from '.';

afterEach(clearMocks);

const select = (stop: string) =>
  useTimelineStore.getState().select({ kind: 'explode', asset: 'switch-cabinet', stop });

const track = () => useTimelineStore.getState().draft.timeline.explode['switch-cabinet'];

describe('the explode key editor', () => {
  it('sets how far apart the parts are at the stop and the spread', async () => {
    await openTimeline();
    select('pullback');
    render(<ExplodeKeyEditor />);
    expect(
      screen.getByRole('heading', { name: 'Switch cabinet · explode at Pullback' })
    ).toBeVisible();
    const amount = screen.getByLabelText('Apart at Pullback');
    await userEvent.clear(amount);
    await userEvent.type(amount, '40{Enter}');
    expect(track()?.keys).toContainEqual({ stop: 'pullback', amount: 0.4 });
    const spread = screen.getByLabelText('Full spread');
    await userEvent.clear(spread);
    await userEvent.type(spread, '0.5{Enter}');
    expect(track()?.distance).toBe(0.5);
  });

  it('says a stop is not keyed yet and removes the track', async () => {
    await openTimeline();
    select('crystals');
    render(<ExplodeKeyEditor />);
    expect(
      screen.getByText('Not keyed at Crystals yet. Set how far apart to key it.')
    ).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Remove the track' }));
    expect(track()).toBeUndefined();
  });
});
