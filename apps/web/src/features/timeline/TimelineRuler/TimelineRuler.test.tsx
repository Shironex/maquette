import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { usePageStore } from '@/state/use-page-store';
import { clearMocks, openTimeline } from '@/test/open-timeline';
import { TimelineRuler } from '.';
import { rulerTicks } from './TimelineRuler.hooks';

const INITIAL = usePageStore.getState();

afterEach(() => {
  clearMocks();
  usePageStore.setState(INITIAL, true);
});

describe('the timeline ruler', () => {
  it('labels every 10 percent, or each section', () => {
    const labels = rulerTicks('percent', 5, null).flatMap(t => (t.label ? [t.label] : []));
    expect(labels).toEqual(['10%', '20%', '30%', '40%', '50%', '60%', '70%', '80%', '90%']);
    expect(rulerTicks('stops', 3, null).map(t => [t.at, t.label])).toEqual([
      [0, '1'],
      [50, '2'],
      [100, '3'],
    ]);
  });

  it('scrubs the page from the keyboard and jumps to a camera stop', async () => {
    await openTimeline();
    render(<TimelineRuler />);
    const slider = screen.getByRole('slider', { name: 'Playhead' });
    expect(slider).toHaveAttribute('aria-valuetext', '0%, at Porch');
    fireEvent.keyDown(slider, { key: 'End' });
    expect(usePageStore.getState().target).toBe(4);
    await userEvent.click(screen.getByRole('button', { name: /^Camera stop 3, Shrubs/ }));
    expect(usePageStore.getState().target).toBe(2);
    expect(usePageStore.getState().smooth).toBe(true);
  });
});
