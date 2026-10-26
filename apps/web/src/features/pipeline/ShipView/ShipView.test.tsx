import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useAgentsStore } from '@/state/use-agents-store';
import { useGameStore } from '@/state/use-game-store';
import { clearMocks, openMockGame } from '@/test/mock-ipc';
import { ShipView } from '.';
import { shipCells, splitPath } from './ShipView.hooks';

beforeEach(async () => {
  await openMockGame('crypt');
  await act(() => useAgentsStore.getState().load());
});

afterEach(() => {
  clearMocks();
  useGameStore.setState({ plan: null });
  useAgentsStore.setState({ status: null });
});

describe('ShipView', () => {
  it('shows what Ship writes and leaves hand made files alone', async () => {
    render(<ShipView project="crypt-game" onClose={vi.fn()} />);
    expect(
      await screen.findByRole('heading', { level: 1, name: 'Ship to Crypt game' })
    ).toBeInTheDocument();
    expect(screen.getByText(/Run 7 · waiting for you · 6 of/)).toBeInTheDocument();
    expect(screen.getByText('3 new · 3 replaced · 0 removed')).toBeInTheDocument();
    expect(screen.getByTitle('characters/heroine.glb')).toBeInTheDocument();
    expect(screen.getByText(/Ship never commits/)).toBeInTheDocument();
  });

  it('approves the ship in the app', async () => {
    render(<ShipView project="crypt-game" onClose={vi.fn()} />);
    await userEvent.click(await screen.findByRole('button', { name: 'Approve and ship' }));
    expect(useAgentsStore.getState().status?.approvals).toEqual([]);
    expect(await screen.findByText(/What the next Ship changes · 0 of/)).toBeInTheDocument();
  });

  it('shows before and after for a replaced file', () => {
    expect(splitPath('props/brazier.glb')).toEqual(['props/', 'brazier.glb']);
    expect(
      shipCells({
        path: 'props/brazier.glb',
        change: 'replaced',
        bytes: 910_000,
        beforeBytes: 1_180_000,
        triangles: 11_800,
        beforeTriangles: 14_600,
        asset: 'brazier',
        blocked: false,
      })
    ).toEqual({ size: '1.2 MB → 910 KB', model: '14.6k → 11.8k tris' });
  });
});
