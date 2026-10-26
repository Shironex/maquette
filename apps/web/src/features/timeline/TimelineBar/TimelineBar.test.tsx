import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { usePageStore } from '@/state/use-page-store';
import { useTimelineStore } from '@/state/use-timeline-store';
import { clearMocks, openTimeline } from '@/test/open-timeline';
import { TimelineBar } from '.';

const INITIAL = usePageStore.getState();

afterEach(() => {
  clearMocks();
  usePageStore.setState(INITIAL, true);
});

describe('the timeline toolbar', () => {
  it('says where the page is and plays it', async () => {
    await openTimeline();
    usePageStore.getState().seek(1.5);
    render(<TimelineBar />);
    expect(screen.getByRole('toolbar')).toHaveTextContent('38% · between Sakura and Shrubs');
    await userEvent.click(screen.getByRole('button', { name: 'Play the scroll at reading speed' }));
    expect(usePageStore.getState().playing).toBe(true);
    await userEvent.click(screen.getByRole('button', { name: 'Go to the top' }));
    expect(usePageStore.getState().target).toBe(0);
  });

  it('adds an explode track and offers paths only as coming next', async () => {
    await openTimeline();
    render(<TimelineBar />);
    await userEvent.click(screen.getByRole('button', { name: 'Add track' }));
    expect(screen.getByRole('menuitem', { name: 'Path' })).toHaveAttribute('aria-disabled', 'true');
    expect(screen.queryByRole('menuitem', { name: 'Explode Switch cabinet' })).toBeNull();
    const items = screen
      .getAllByRole('menuitem')
      .filter(i => /^Explode /.test(i.textContent ?? ''));
    const open = items.find(i => !i.hasAttribute('aria-disabled') && !i.hasAttribute('disabled'));
    expect(open).toBeDefined();
    await userEvent.click(open!);
    expect(useTimelineStore.getState().selection?.kind).toBe('explode');
  });

  it('keeps Pixels off until the page answers with its scroll', async () => {
    await openTimeline();
    render(<TimelineBar />);
    expect(screen.getByRole('button', { name: 'Pixels' })).toHaveAttribute('aria-disabled', 'true');
    await userEvent.click(screen.getByRole('button', { name: 'Sections' }));
    expect(useTimelineStore.getState().units).toBe('stops');
  });
});
