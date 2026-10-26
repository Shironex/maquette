import { act, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { usePageStore } from '@/state/use-page-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { CompareBoard } from '.';

const INITIAL = usePageStore.getState();

afterEach(() => {
  clearMocks();
  usePageStore.setState(INITIAL, true);
});

async function open() {
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
  await usePageStore.getState().open('moon-island');
  await vi.waitFor(() => expect(usePageStore.getState().inspect).not.toBeNull());
  await vi.waitFor(async () => {
    await usePageStore.getState().refresh();
    expect(usePageStore.getState().view?.server.state).toBe('running');
  });
}

describe('the compare board', () => {
  it('puts the current ship beside this run', async () => {
    await open();
    act(() => usePageStore.getState().setMode('ship-run'));
    render(<CompareBoard />);
    const ship = screen.getByRole('region', { name: 'Current ship' });
    expect(within(ship).getByTitle(/^Current ship, /)).toBeInTheDocument();
    const run = screen.getByRole('region', { name: 'This run' });
    expect(within(run).getByText('Ships next')).toBeVisible();
    expect(within(run).getByText('+1.4 MB')).toBeVisible();
    expect(screen.queryByRole('region', { name: 'Poster fallback' })).toBeNull();
  });

  it('adds the page without WebGL with poster', async () => {
    await open();
    act(() => usePageStore.getState().setMode('poster'));
    render(<CompareBoard />);
    const poster = screen.getByRole('region', { name: 'Poster fallback' });
    expect(within(poster).getByText('no WebGL')).toBeVisible();
    expect(within(poster).getByText('Poster')).toBeVisible();
  });

  it('shows the frame time the ship page sends', async () => {
    await open();
    act(() => usePageStore.getState().setMode('ship-run'));
    render(<CompareBoard />);
    act(() =>
      usePageStore
        .getState()
        .onMessage('ship', { type: 'stats', frameMs: 9.4, p95Ms: 12.1, drawCalls: 86 })
    );
    const ship = screen.getByRole('region', { name: 'Current ship' });
    expect(within(ship).getByText('12.1 ms')).toBeVisible();
    expect(within(ship).getByText('86')).toBeVisible();
  });
});
