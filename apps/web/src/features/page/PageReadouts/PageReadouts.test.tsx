import { act, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { usePageStore } from '@/state/use-page-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { PageReadouts } from '.';

const INITIAL = usePageStore.getState();

afterEach(() => {
  clearMocks();
  usePageStore.setState(INITIAL, true);
});

describe('the page readouts', () => {
  it('waits for the page to send its frame times', async () => {
    await openMockWorkspace(
      'configured',
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      'running'
    );
    await usePageStore.getState().open('frostveil');
    await vi.waitFor(() => expect(usePageStore.getState().inspect).not.toBeNull());
    render(<PageReadouts />);
    expect(screen.getByText('not sent')).toBeVisible();
    expect(screen.getByText(/The page sends no frame times yet/)).toBeVisible();
    expect(screen.getByText('Current ship')).toBeVisible();
  });

  it('shows what the page measures and flags a slow frame', async () => {
    await openMockWorkspace();
    usePageStore.setState({ device: 'phone' });
    render(<PageReadouts />);
    act(() => {
      const { onMessage } = usePageStore.getState();
      onMessage('main', {
        source: 'site',
        type: 'ready',
        stops: [{ name: 'Arrival' }, { name: 'Veil gate' }],
        features: [],
      });
      onMessage('main', { source: 'site', type: 'at', stop: 1, scroll: null });
      onMessage('main', {
        source: 'site',
        type: 'stats',
        frameMs: 21.4,
        p95Ms: 38.2,
        drawCalls: 71,
      });
    });
    expect(screen.getByText('21.4 ms')).toHaveClass('text-danger');
    expect(screen.getByText('p95 38.2')).toBeVisible();
    expect(screen.getByText('Draw calls')).toBeVisible();
    expect(screen.getByText(/Measured by the site's page bridge at stop 2 on Phone/)).toBeVisible();
  });

  it("says when Maquette's shim measures a page without a bridge", async () => {
    await openMockWorkspace();
    render(<PageReadouts />);
    act(() => {
      const { onMessage } = usePageStore.getState();
      onMessage('main', {
        source: 'shim',
        type: 'ready',
        stops: [{ name: 'Arrival' }],
        features: [],
      });
      onMessage('main', { source: 'shim', type: 'at', stop: 0, scroll: null });
      onMessage('main', {
        source: 'shim',
        type: 'stats',
        frameMs: 16.7,
        p95Ms: 18.2,
        drawCalls: null,
      });
    });
    expect(screen.getByText('16.7 ms')).toBeVisible();
    expect(screen.getByText('Draw calls, est.')).toBeVisible();
    expect(screen.getByText(/Measured by Maquette's shim at stop 1 on Desktop/)).toBeVisible();
    expect(screen.getByText(/the site has no page bridge/i)).toBeVisible();
  });
});
