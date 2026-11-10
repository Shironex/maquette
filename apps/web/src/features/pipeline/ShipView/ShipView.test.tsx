import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
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
    expect(screen.getByText(/Run 7 · waiting for you · 5 of/)).toBeInTheDocument();
    expect(screen.getByText('3 new · 2 replaced · 0 removed')).toBeInTheDocument();
    expect(screen.getByTitle('characters/heroine.glb')).toBeInTheDocument();
    expect(screen.getByText(/Ship never commits/)).toBeInTheDocument();
  });

  it('names the models a built asset replaced since the last Ship', async () => {
    vi.spyOn(commands, 'builtSwaps').mockResolvedValue([
      {
        asset: 'brazier',
        name: 'Brazier',
        script: 'built/brazier.py',
        at: '2026-11-10T09:30:00.000Z',
        oldFile: 'props/brazier.glb',
        newFile: 'props/brazier.glb',
        oldVersion: 1,
        newVersion: 2,
        oldTriangles: 41_000,
        newTriangles: 1_840,
        oldBytes: 1_900_000,
        newBytes: 96_000,
      },
    ]);
    render(<ShipView project="crypt-game" onClose={vi.fn()} />);
    const swaps = await screen.findByRole('region', {
      name: 'Built models that replace another model in this Ship',
    });
    expect(swaps).toHaveTextContent(
      'Brazier is a built model now: props/brazier.glb · 1.8k triangles, was 41k · 96 KB, was 1.9 MB'
    );
    vi.restoreAllMocks();
  });

  it('says which replaced files are the imported originals and to commit first', async () => {
    render(<ShipView project="crypt-game" onClose={vi.fn()} />);
    await screen.findByRole('heading', { level: 1, name: 'Ship to Crypt game' });
    expect(screen.getAllByText('replaces the imported original')).toHaveLength(2);
    expect(screen.getByText(/2 replace the imported originals/)).toBeInTheDocument();
    expect(screen.getByRole('note')).toHaveTextContent(
      'The game repo has 3 files not committed yet. Commit them first'
    );
  });

  it('approves the ship in the app', async () => {
    const decide = vi.spyOn(commands, 'mcpDecide');
    render(<ShipView project="crypt-game" onClose={vi.fn()} />);
    await userEvent.click(await screen.findByRole('button', { name: 'Approve and ship' }));
    expect(decide).toHaveBeenCalledWith(
      expect.any(String),
      true,
      expect.objectContaining({ source: 'ship-view', input: 'pointer' })
    );
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
        imported: true,
      })
    ).toEqual({ size: '1.2 MB → 910 KB', model: '14.6k → 11.8k tris' });
  });
});
