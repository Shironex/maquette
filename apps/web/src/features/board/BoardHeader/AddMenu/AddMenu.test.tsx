import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { forgetStoredPrefs, useBoardPrefsStore } from '@/state/use-board-prefs-store';
import { useBuiltStore } from '@/state/use-built-store';
import { useGameStore } from '@/state/use-game-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useTrashStore } from '@/state/use-trash-store';
import { clearMocks, openMockGame, openMockWorkspace } from '@/test/mock-ipc';
import AddMenu from './AddMenu';

const openAdd = () => userEvent.click(screen.getByRole('button', { name: 'More ways to add' }));

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, 'batch');
  await useProvidersStore.getState().load('frostveil');
});

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({
    project: null,
    jobs: [],
    queueOpen: false,
    checklistOpen: false,
    busy: [],
  });
  useTrashStore.setState({ project: null, entries: [], restoring: null });
  useAppStore.setState({ notice: null });
  window.localStorage.clear();
  forgetStoredPrefs();
  useBoardPrefsStore.setState({ byProject: {} });
});

describe('the Add split button', () => {
  it('toggles the add asset form on its main click', async () => {
    render(<AddMenu />);
    const add = screen.getByRole('button', { name: 'Add asset' });
    expect(add).toHaveTextContent('Add');
    await userEvent.click(add);
    expect(screen.getByRole('form', { name: 'Add asset' })).toBeInTheDocument();
    expect(add).toHaveAttribute('aria-expanded', 'true');
    await userEvent.click(add);
    expect(screen.queryByRole('form', { name: 'Add asset' })).toBeNull();
  });

  it('lists only Built asset on a website: the main button already adds an asset', async () => {
    render(<AddMenu />);
    await openAdd();
    const menu = screen.getByRole('menu', { name: 'Add to the project' });
    expect(
      within(menu)
        .getAllByRole('menuitem')
        .map(i => i.textContent)
    ).toEqual(['Built asset']);
    expect(within(menu).queryByRole('menuitem', { name: 'Add asset' })).toBeNull();
  });

  it('adds Import models on a game and opens the import', async () => {
    await openMockGame('crypt');
    const before = useGameStore.getState().openImport;
    const open = vi.fn(async () => {});
    useGameStore.setState({ openImport: open });
    render(<AddMenu />);
    await openAdd();
    const menu = screen.getByRole('menu', { name: 'Add to the project' });
    expect(
      within(menu)
        .getAllByRole('menuitem')
        .map(i => i.textContent)
    ).toEqual(['Import models', 'Built asset']);
    await userEvent.click(within(menu).getByRole('menuitem', { name: 'Import models' }));
    expect(open).toHaveBeenCalledWith(useAppStore.getState().workspace!.project!.id);
    useGameStore.setState({ openImport: before });
  });

  it('opens Add built asset from the menu and closes the add form', async () => {
    render(<AddMenu />);
    await userEvent.click(screen.getByRole('button', { name: 'Add asset' }));
    await openAdd();
    await userEvent.click(screen.getByRole('menuitem', { name: 'Built asset' }));
    expect(useBuiltStore.getState().adding).toBe(true);
    expect(screen.queryByRole('form', { name: 'Add asset' })).toBeNull();
    useBuiltStore.getState().setAdding(false);
  });

  it('closes on a second press of the caret and on Escape, handing focus back', async () => {
    render(<AddMenu />);
    await openAdd();
    await openAdd();
    expect(screen.queryByRole('menu')).toBeNull();
    await openAdd();
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('menu')).toBeNull();
    expect(screen.getByRole('button', { name: 'More ways to add' })).toHaveFocus();
  });
});
