import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { usePageStore } from '@/state/use-page-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { ShipTable } from '.';

const INITIAL = usePageStore.getState();

afterEach(() => {
  clearMocks();
  usePageStore.setState(INITIAL, true);
});

async function open(page?: string) {
  await openMockWorkspace(
    'configured',
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    page
  );
  await usePageStore.getState().open('moon-island');
  await vi.waitFor(() => expect(usePageStore.getState().inspect).not.toBeNull());
}

describe('the ship table', () => {
  it('compares the site models with this run', async () => {
    await open('running');
    render(<ShipTable />);
    const size = screen.getByRole('row', { name: /Scene size/ });
    expect(within(size).getByText('0.6 MB')).toBeVisible();
    expect(within(size).getByText('2.0 MB')).toBeVisible();
    expect(within(size).getByText('+1.4 MB')).toHaveClass('text-danger');
  });

  it('says when nothing is on the site yet', async () => {
    await open();
    render(<ShipTable />);
    expect(
      screen.getByText('Nothing is on the site yet: Ship copies this run there.')
    ).toBeVisible();
  });

  it('opens the compare board', async () => {
    await open('running');
    render(<ShipTable />);
    await userEvent.click(screen.getByRole('button', { name: 'Compare' }));
    expect(usePageStore.getState().mode).toBe('ship-run');
  });
});
