import { act, fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { usePageStore } from '@/state/use-page-store';
import { StopScrubber } from '.';

const INITIAL = usePageStore.getState();

afterEach(() => {
  usePageStore.setState(INITIAL, true);
});

function answer() {
  const { onMessage } = usePageStore.getState();
  act(() => {
    onMessage('main', {
      type: 'ready',
      stops: [{ name: 'Hero' }, { name: 'Services' }, { name: 'Switch room' }, { name: 'Contact' }],
      features: [],
    });
    onMessage('main', { type: 'at', stop: 1, scroll: { top: 1850, max: 5960 } });
  });
}

describe('the stop scrubber', () => {
  it('says where the page is, from the page itself', () => {
    render(<StopScrubber />);
    answer();
    expect(screen.getByText(/1,850 of 5,960 px · stop 2, Services/)).toBeVisible();
    expect(screen.getByRole('button', { name: 'Jump to stop 2, Services' })).toHaveAttribute(
      'aria-current',
      'step'
    );
  });

  it('seeks every frame to a stop and steps through them', async () => {
    render(<StopScrubber />);
    answer();
    await userEvent.click(screen.getByRole('button', { name: 'Jump to stop 4, Contact' }));
    expect(usePageStore.getState().target).toBe(3);
    await userEvent.click(screen.getByRole('button', { name: 'Previous stop' }));
    expect(usePageStore.getState().target).toBe(0);
    fireEvent.change(screen.getByRole('slider', { name: 'Scroll position' }), {
      target: { value: '2.5' },
    });
    expect(usePageStore.getState().target).toBe(2.5);
  });

  it('falls back to the camera stops until the page answers', () => {
    usePageStore.setState({
      inspect: {
        stops: {
          source: 'project-file',
          file: 'stops.json',
          stops: [{ name: 'Porch' }, { name: 'Sakura' }],
        },
      } as never,
    });
    render(<StopScrubber />);
    expect(screen.getByText(/has not answered yet/)).toBeVisible();
    expect(screen.getByRole('button', { name: 'Jump to stop 2, Sakura' })).toBeVisible();
  });

  it('plays and pauses', async () => {
    render(<StopScrubber />);
    answer();
    await userEvent.click(screen.getByRole('button', { name: 'Play the scroll at reading speed' }));
    expect(usePageStore.getState().playing).toBe(true);
    await userEvent.click(screen.getByRole('button', { name: 'Pause' }));
    expect(usePageStore.getState().playing).toBe(false);
  });
});
