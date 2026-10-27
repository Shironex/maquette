import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useTimelineStore } from '@/state/use-timeline-store';
import { clearMocks, openTimeline } from '@/test/open-timeline';
import { StateKeyEditor } from '.';

afterEach(clearMocks);

const select = (index: number) =>
  useTimelineStore.getState().select({ kind: 'state', asset: 'switch-cabinet', index });

describe('the state change editor', () => {
  it('reads a stop change with the move the Motion view set', async () => {
    await openTimeline();
    select(0);
    render(<StateKeyEditor />);
    expect(screen.getByRole('heading', { name: 'Switch cabinet · Door open' })).toBeVisible();
    expect(screen.getByLabelText('At the stop')).toHaveValue('veil-gate');
    expect(screen.getByText('Plays once, 0.6 s, when the camera arrives')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Time' })).toHaveAttribute('aria-disabled', 'true');
  });

  it('turns a stop change into a scroll range and edits its end', async () => {
    await openTimeline();
    select(0);
    render(<StateKeyEditor />);
    await userEvent.click(screen.getByRole('button', { name: 'Scroll range' }));
    expect(useTimelineStore.getState().draft.triggers['switch-cabinet']?.[0]).toMatchObject({
      kind: 'scroll',
      start: 0,
      end: 25,
    });
    const end = screen.getByLabelText('Ends');
    await userEvent.clear(end);
    await userEvent.type(end, '40{Enter}');
    expect(useTimelineStore.getState().draft.triggers['switch-cabinet']?.[0]?.end).toBe(40);
  });

  it('removes the change', async () => {
    await openTimeline();
    select(1);
    render(<StateKeyEditor />);
    await userEvent.click(screen.getByRole('button', { name: 'Remove this change' }));
    const s = useTimelineStore.getState();
    expect(s.draft.triggers['switch-cabinet']).toHaveLength(1);
    expect(s.selection).toBeNull();
  });
});
