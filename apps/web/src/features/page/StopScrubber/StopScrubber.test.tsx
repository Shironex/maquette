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
      source: 'site',
      type: 'ready',
      stops: [{ name: 'Hero' }, { name: 'Services' }, { name: 'Switch room' }, { name: 'Contact' }],
      features: [],
    });
    onMessage('main', { source: 'site', type: 'at', stop: 1, scroll: { top: 1850, max: 5960 } });
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

  it('says which bridge moves the page', () => {
    const { onMessage } = usePageStore.getState();
    render(<StopScrubber />);
    expect(screen.getByText(/has not answered yet/)).toBeVisible();
    act(() =>
      onMessage('main', { source: 'shim', type: 'ready', stops: [{ name: 'Porch' }], features: [] })
    );
    expect(
      screen.getByText("Maquette's shim scrolls the page: the site has no page bridge")
    ).toBeVisible();
    act(() =>
      onMessage('main', { source: 'site', type: 'ready', stops: [{ name: 'Hero' }], features: [] })
    );
    expect(screen.getByText("The site's page bridge scrolls the page")).toBeVisible();
  });

  it('keeps the camera stops while the shim has none to say back', () => {
    usePageStore.setState({
      inspect: {
        stops: { source: 'project-file', file: 'stops.json', stops: [{ name: 'Porch' }] },
      } as never,
    });
    render(<StopScrubber />);
    act(() =>
      usePageStore
        .getState()
        .onMessage('main', { source: 'shim', type: 'ready', stops: [], features: [] })
    );
    expect(screen.getByRole('button', { name: 'Jump to stop 1, Porch' })).toBeVisible();
  });

  it('plays and pauses', async () => {
    render(<StopScrubber />);
    answer();
    await userEvent.click(screen.getByRole('button', { name: 'Play the scroll at reading speed' }));
    expect(usePageStore.getState().playing).toBe(true);
    await userEvent.click(screen.getByRole('button', { name: 'Pause' }));
    expect(usePageStore.getState().playing).toBe(false);
  });

  it('keeps ten stops apart, each cut to its slot with its full name on hover', () => {
    const names = [
      'Hero',
      'Lesson plans',
      'Subject chemistry',
      'Subject physics',
      'Exam prep',
      'Study room',
      'Pricing',
      'Tutors',
      'Testimonials',
      'Contact',
    ];
    const { onMessage } = usePageStore.getState();
    render(<StopScrubber />);
    act(() =>
      onMessage('main', {
        source: 'site',
        type: 'ready',
        stops: names.map(name => ({ name })),
        features: [],
      })
    );
    const marks = names.map((name, i) =>
      screen.getByRole('button', { name: `Jump to stop ${i + 1}, ${name}` })
    );
    expect(marks.map(m => m.getAttribute('title'))).toEqual(names);
    expect(marks[2]!.style.width).toBe('calc(11.11% - 6px)');
    expect(marks[8]!.style.width).toBe('calc(5.56% - 6px)');
    expect(marks[9]!.className).toContain('right-0');
    expect(marks[9]!.style.width).toBe('calc(5.56% + 5px)');
    expect(marks[9]!.querySelector('.truncate')).toHaveTextContent('Contact');
  });
});
