import { act, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { usePageStore } from '@/state/use-page-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { DevicesBoard } from '.';

const INITIAL = usePageStore.getState();

afterEach(() => {
  clearMocks();
  usePageStore.setState(INITIAL, true);
});

describe('the devices board', () => {
  it('shows the page at every device width, each with its own frame time', async () => {
    await openMockWorkspace(
      'configured',
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      'running'
    );
    await usePageStore.getState().open('moon-island');
    await vi.waitFor(async () => {
      await usePageStore.getState().refresh();
      expect(usePageStore.getState().view?.server.state).toBe('running');
    });
    render(<DevicesBoard />);
    for (const name of ['Phone', 'Laptop', 'Desktop']) {
      const column = screen.getByRole('region', { name });
      expect(within(column).getByTitle(`The page at ${name} width`)).toBeInTheDocument();
    }
    act(() =>
      usePageStore
        .getState()
        .onMessage('phone', { type: 'stats', frameMs: 21.4, p95Ms: 38.2, drawCalls: 71 })
    );
    expect(
      within(screen.getByRole('region', { name: 'Phone' })).getByText('p95 38.2 ms')
    ).toBeVisible();
    expect(
      within(screen.getByRole('region', { name: 'Laptop' })).getByText('no frame time yet')
    ).toBeVisible();
  });
});
