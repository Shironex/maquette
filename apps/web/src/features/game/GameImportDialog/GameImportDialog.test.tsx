import { act, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useGameStore } from '@/state/use-game-store';
import { clearMocks, openMockGame } from '@/test/mock-ipc';
import { GameImportDialog } from '.';

beforeEach(async () => {
  await openMockGame('import');
  await act(() => useGameStore.getState().openImport('crypt-game'));
});

afterEach(() => {
  clearMocks();
  useGameStore.setState({ importFor: null, scan: null, picked: [] });
});

describe('GameImportDialog', () => {
  it('lists the scanned models with every one picked', () => {
    render(<GameImportDialog />);
    const dialog = screen.getByRole('dialog', { name: /Import models from Crypt game/ });
    expect(within(dialog).getByTitle('props/iron_gate.glb')).toHaveTextContent('iron_gate.glb');
    expect(
      within(dialog).getByTitle('Over 12k triangles, Texture over 1024 px')
    ).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: 'Import 16 models' })).toBeInTheDocument();
  });

  it('imports the picked models; the ones left out stay hand made', async () => {
    render(<GameImportDialog />);
    await userEvent.click(screen.getByRole('checkbox', { name: 'Import characters/heroine.glb' }));
    await userEvent.click(screen.getByRole('button', { name: 'Import 15 models' }));
    const project = useAppStore.getState().workspace?.project;
    expect(project?.assets).toHaveLength(15);
    expect(project?.game?.models['iron-gate']).toEqual({
      file: 'props/iron_gate.glb',
      importedVersion: 1,
    });
    expect(project?.game?.models.heroine).toBeUndefined();
    expect(useGameStore.getState().importFor).toBeNull();
  });

  it('asks for a preview of every model it imports', async () => {
    render(<GameImportDialog />);
    await userEvent.click(screen.getByRole('button', { name: 'Import 16 models' }));
    const assets = useAppStore.getState().workspace?.project?.assets ?? [];
    expect(assets).toHaveLength(16);
    expect(assets.every(a => a.thumbnail?.startsWith('thumbs/'))).toBe(true);
    expect(useAppStore.getState().notice?.detail).toMatch(/preview/);
  });
});
