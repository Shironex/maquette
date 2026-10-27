import { act, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { usePageStore } from '@/state/use-page-store';
import { PageFrame } from '.';

const INITIAL = usePageStore.getState();

afterEach(() => {
  usePageStore.setState(INITIAL, true);
});

function frame() {
  render(
    <PageFrame
      id="main"
      src="http://localhost:4321/"
      address="localhost:4321/"
      badge="live"
      device="desktop"
      label="The home page from the site dev page"
      showStop
    />
  );
  return screen.getByTitle('The home page from the site dev page') as HTMLIFrameElement;
}

function fromPage(iframe: HTMLIFrameElement, data: unknown) {
  act(() => {
    window.dispatchEvent(new MessageEvent('message', { data, source: iframe.contentWindow }));
  });
}

describe('the page frame', () => {
  it('shows the page in a sandbox behind its address bar', () => {
    const iframe = frame();
    expect(iframe).toHaveAttribute('src', 'http://localhost:4321/');
    expect(iframe.getAttribute('sandbox')).not.toContain('allow-top-navigation');
    expect(screen.getByText('localhost:4321/')).toBeVisible();
    expect(screen.getByText('Live')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Reload the page' })).toBeEnabled();
  });

  it('keeps what its page says and shows the current stop', () => {
    const iframe = frame();
    fromPage(iframe, {
      maquette: 1,
      type: 'ready',
      stops: [{ name: 'Arrival' }, { name: 'Veil gate' }],
    });
    fromPage(iframe, { maquette: 1, type: 'at', stop: 0.9, scroll: { top: 700, max: 3000 } });
    expect(usePageStore.getState().readings.main?.at).toBe(0.9);
    expect(screen.getByText('Veil gate')).toBeVisible();
  });

  it('hears only its own page', () => {
    frame();
    act(() => {
      window.dispatchEvent(
        new MessageEvent('message', { data: { maquette: 1, type: 'at', stop: 3 }, source: window })
      );
    });
    expect(usePageStore.getState().readings.main?.at ?? null).toBeNull();
  });

  it('says hello and sends each seek to its page', () => {
    const iframe = frame();
    const post = vi.spyOn(iframe.contentWindow!, 'postMessage');
    act(() => usePageStore.getState().seek(2.5, true));
    expect(post).toHaveBeenCalledWith({ maquette: 1, type: 'goto', stop: 2.5, smooth: true }, '*');
  });

  it("says hello with the camera stops and keeps on after the shim, until the site's bridge", () => {
    vi.useFakeTimers();
    usePageStore.setState({
      inspect: {
        stops: { source: 'project-file', file: 'stops.json', stops: [{ name: 'Arrival' }] },
      } as never,
    });
    const iframe = frame();
    const post = vi.spyOn(iframe.contentWindow!, 'postMessage');
    const hellos = () => post.mock.calls.filter(([m]) => (m as { type: string }).type === 'hello');
    const hello = { maquette: 1, type: 'hello', stops: [{ name: 'Arrival' }] };
    act(() => vi.advanceTimersByTime(500));
    expect(post).toHaveBeenCalledWith(hello, '*');
    fromPage(iframe, { maquette: 1, source: 'shim', type: 'ready', stops: [{ name: 'Arrival' }] });
    const afterShim = hellos().length;
    act(() => vi.advanceTimersByTime(1000));
    expect(hellos().length).toBe(afterShim + 2);
    fromPage(iframe, { maquette: 1, type: 'ready', stops: [{ name: 'Hero' }] });
    act(() => vi.advanceTimersByTime(1000));
    expect(hellos().length).toBe(afterShim + 2);
    expect(usePageStore.getState().readings.main?.source).toBe('site');
    vi.useRealTimers();
  });

  it('shows its children while there is no page', () => {
    render(
      <PageFrame id="main" src={null} address="" badge="live" device="phone" label="No page">
        <p>Starting the dev server</p>
      </PageFrame>
    );
    expect(screen.getByText('Starting the dev server')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Reload the page' })).toBeDisabled();
  });
});
