import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useTimelineStore } from '@/state/use-timeline-store';
import { clearMocks, openTimeline } from '@/test/open-timeline';
import { StateLane } from '.';

afterEach(clearMocks);

describe('a state lane', () => {
  it('shows the stop change and the scroll range, and selects one', async () => {
    await openTimeline();
    render(<StateLane asset="switch-cabinet" />);
    expect(screen.getByRole('button', { name: 'Switch cabinet: Door open at 25%' })).toBeVisible();
    await userEvent.click(
      screen.getByRole('button', { name: 'Switch cabinet: Breakers on from 55% to 70%' })
    );
    expect(useTimelineStore.getState().selection).toEqual({
      kind: 'state',
      asset: 'switch-cabinet',
      index: 1,
    });
  });

  it('adds a change at a free stop', async () => {
    await openTimeline();
    render(<StateLane asset="switch-cabinet" />);
    expect(screen.queryByRole('button', { name: 'Change Switch cabinet at Veil gate' })).toBeNull();
    await userEvent.click(
      screen.getByRole('button', { name: 'Change Switch cabinet at Pullback' })
    );
    const { draft, selection } = useTimelineStore.getState();
    expect(draft.triggers['switch-cabinet']?.[2]).toMatchObject({ kind: 'stop', stop: 'pullback' });
    expect(selection).toEqual({ kind: 'state', asset: 'switch-cabinet', index: 2 });
  });
});
