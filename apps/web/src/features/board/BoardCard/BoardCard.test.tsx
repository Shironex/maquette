import { fireEvent, render, screen } from '@testing-library/react';
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
  await useProvidersStore.getState().load('frostveil');
});

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ project: null, jobs: [], batch: null, batchesSent: 0, busy: [] });
});

describe('BoardCard', () => {
  it('shows the asset with its latest version and checks its box', async () => {
    const board = fakeBoardEnv();
    render(<BoardCard asset={mockAsset('Elsheria')} board={board} />);
    expect(screen.getByRole('button', { name: 'Elsheria, v2' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Elsheria' }));
    expect(board.selection.check).toHaveBeenCalledWith('elsheria', true, false);
  });

  it('opens its menu from the dots, closing any copy list', async () => {
    const board = fakeBoardEnv();
    const { rerender } = render(<BoardCard asset={mockAsset('Elsheria')} board={board} />);
    await userEvent.click(screen.getByRole('button', { name: 'More for Elsheria' }));
    expect(board.actions.setCopyFor).toHaveBeenCalledWith(null);
    expect(board.actions.setMenuFor).toHaveBeenCalledOnce();
    expect(screen.queryByRole('menu')).toBeNull();
    const open = fakeBoardEnv({ actions: { ...board.actions, menuFor: 'elsheria' } });
    rerender(<BoardCard asset={mockAsset('Elsheria')} board={open} />);
    expect(screen.getByRole('menu', { name: 'Elsheria actions' })).toBeInTheDocument();
  });

  it('asks for the model of a waiting By hand card', async () => {
    const addModel = vi.fn(async () => {});
    useProvidersStore.setState({ addModel });
    render(<BoardCard asset={mockAsset('Sigil door')} board={fakeBoardEnv()} />);
    expect(screen.getByText('Waiting for your model')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Add model...' }));
    expect(addModel).toHaveBeenCalledWith('sigil-door-1');
  });

  it('shows the export name a waiting card wants, with its copy button', () => {
    render(<BoardCard asset={mockAsset('Sigil door')} board={fakeBoardEnv()} />);
    const name = useProvidersStore.getState().jobs.find(j => j.id === 'sigil-door-1')
      ?.card?.exportName;
    expect(name).toMatch(/^maq_sigil-door_v1_/);
    expect(screen.getByText(name!)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Copy export name' })).toBeInTheDocument();
  });

  it('has no export name line without a waiting send', () => {
    render(<BoardCard asset={mockAsset('Elsheria')} board={fakeBoardEnv()} />);
    expect(screen.queryByRole('button', { name: 'Copy export name' })).toBeNull();
  });

  it('opens its menu on a right-click', () => {
    const board = fakeBoardEnv();
    render(<BoardCard asset={mockAsset('Elsheria')} board={board} />);
    const card = screen.getByRole('button', { name: 'Elsheria, v2' });
    expect(fireEvent.contextMenu(card)).toBe(false);
    expect(board.actions.setMenuFor).toHaveBeenCalledWith('elsheria');
    expect(board.actions.setCopyFor).toHaveBeenCalledWith(null);
  });
});

describe('BoardCard of a built asset', () => {
  beforeEach(async () => {
    await openMockWorkspace('configured', undefined, undefined, undefined, undefined, 'demo');
    await useBuiltStore.getState().load('frostveil');
  });
  afterEach(() => useBuiltStore.setState({ project: null, items: [], adding: false }));

  it('says its script changed and rebuilds from its button', async () => {
    const rebuild = vi.fn(async () => {});
    useBuiltStore.setState({ rebuild });
    render(<BoardCard asset={mockAsset('Newtons cradle')} board={fakeBoardEnv()} />);
    expect(screen.getByText('Script changed')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Rebuild' }));
    expect(rebuild).toHaveBeenCalledWith('frostveil', 'newtons-cradle', true);
  });

  it('shows a current one as done, without a Rebuild button', () => {
    render(<BoardCard asset={mockAsset('Laptop')} board={fakeBoardEnv()} />);
    expect(screen.queryByText('Script changed')).toBeNull();
    expect(screen.queryByRole('button', { name: 'Rebuild' })).toBeNull();
  });

  it('shows an update waiting in the library and opens its plan', async () => {
    await openMockLibrary('demo');
    await useLibraryStore.getState().load();
    render(<BoardCard asset={mockAsset('Elsheria')} board={fakeBoardEnv()} />);
    await userEvent.click(screen.getByRole('button', { name: 'Library has v3' }));
    expect(useLibraryStore.getState().updating).toEqual({
      project: 'frostveil',
      asset: 'elsheria',
    });
    useLibraryStore.setState({ view: null, updating: null, plan: null });
  });
});
