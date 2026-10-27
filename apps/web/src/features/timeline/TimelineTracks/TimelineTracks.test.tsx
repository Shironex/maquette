import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { usePageStore } from '@/state/use-page-store';
import { clearMocks, openTimeline } from '@/test/open-timeline';
import { TimelineTracks } from '.';

const INITIAL = usePageStore.getState();

afterEach(() => {
  clearMocks();
  usePageStore.setState(INITIAL, true);
});

describe('the timeline tracks', () => {
  it('jumps a stop with ] and [ and plays with Space', async () => {
    await openTimeline();
    render(<TimelineTracks />);
    fireEvent.keyDown(window, { key: ']' });
    expect(usePageStore.getState().target).toBe(1);
    fireEvent.keyDown(window, { key: ']' });
    fireEvent.keyDown(window, { key: '[' });
    expect(usePageStore.getState().target).toBe(1);
    fireEvent.keyDown(window, { key: ' ' });
    expect(usePageStore.getState().playing).toBe(true);
  });

  it('gives each energized path a lane', async () => {
    await openTimeline();
    render(<TimelineTracks />);
    expect(screen.getByRole('group', { name: 'Rune line energize' })).toBeVisible();
    expect(screen.getByRole('group', { name: 'Root glow energize' })).toBeVisible();
    expect(screen.queryByRole('group', { name: 'Spare run energize' })).toBeNull();
  });
});
