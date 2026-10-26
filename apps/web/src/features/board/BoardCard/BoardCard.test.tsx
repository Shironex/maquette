import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useBuiltStore } from '@/state/use-built-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useLibraryStore } from '@/state/use-library-store';
import { clearMocks, openMockLibrary, openMockWorkspace } from '@/test/mock-ipc';
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

describe('BoardCard of a built asset', () => {
  beforeEach(async () => {
    await openMockWorkspace('configured', undefined, undefined, undefined, undefined, 'demo');
    await useBuiltStore.getState().load('moon-island');
  });
  afterEach(() => useBuiltStore.setState({ project: null, items: [], adding: false }));

  it('says its script changed and rebuilds from its button', async () => {
    const rebuild = vi.fn(async () => {});
    useBuiltStore.setState({ rebuild });
    render(<BoardCard asset={mockAsset('Newtons cradle')} board={fakeBoardEnv()} />);
    expect(screen.getByText('Script changed')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Rebuild' }));
    expect(rebuild).toHaveBeenCalledWith('moon-island', 'newtons-cradle', true);
  });

  it('shows a current one as done, without a Rebuild button', () => {
    render(<BoardCard asset={mockAsset('Laptop')} board={fakeBoardEnv()} />);
    expect(screen.queryByText('Script changed')).toBeNull();
    expect(screen.queryByRole('button', { name: 'Rebuild' })).toBeNull();
  });

  it('shows an update waiting in the library and opens its plan', async () => {
    await openMockLibrary('demo');
    await useLibraryStore.getState().load();
    render(<BoardCard asset={mockAsset('Character')} board={fakeBoardEnv()} />);
    await userEvent.click(screen.getByRole('button', { name: 'Update to v3' }));
    expect(useLibraryStore.getState().updating).toEqual({
      project: 'moon-island',
      asset: 'character',
    });
    useLibraryStore.setState({ view: null, updating: null, plan: null });
  });
});
