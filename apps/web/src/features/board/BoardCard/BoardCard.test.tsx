import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { fakeBoardEnv, mockAsset } from '../fakeBoardEnv';
import BoardCard from './BoardCard';

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, 'batch');
  await useProvidersStore.getState().load('moon-island');
});

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ project: null, jobs: [], batch: null, batchesSent: 0, busy: [] });
});

describe('BoardCard', () => {
  it('shows the asset with its latest version and checks its box', async () => {
    const board = fakeBoardEnv();
    render(<BoardCard asset={mockAsset('Character')} board={board} />);
    expect(screen.getByRole('button', { name: 'Character, v2' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Character' }));
    expect(board.selection.check).toHaveBeenCalledWith('character', true, false);
  });

  it('opens its menu from the dots, closing any copy list', async () => {
    const board = fakeBoardEnv();
    const { rerender } = render(<BoardCard asset={mockAsset('Character')} board={board} />);
    await userEvent.click(screen.getByRole('button', { name: 'More for Character' }));
    expect(board.actions.setCopyFor).toHaveBeenCalledWith(null);
    expect(board.actions.setMenuFor).toHaveBeenCalledOnce();
    expect(screen.queryByRole('menu')).toBeNull();
    const open = fakeBoardEnv({ actions: { ...board.actions, menuFor: 'character' } });
    rerender(<BoardCard asset={mockAsset('Character')} board={open} />);
    expect(screen.getByRole('menu', { name: 'Character actions' })).toBeInTheDocument();
  });

  it('asks for the model of a waiting By hand card', async () => {
    const addModel = vi.fn(async () => {});
    useProvidersStore.setState({ addModel });
    render(<BoardCard asset={mockAsset('Brush cup')} board={fakeBoardEnv()} />);
    expect(screen.getByText('Waiting for your model')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Add model...' }));
    expect(addModel).toHaveBeenCalledWith('brush-cup-1');
  });
});
