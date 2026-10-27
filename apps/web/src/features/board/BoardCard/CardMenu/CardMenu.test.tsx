import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { BuiltAsset } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { useBuiltStore } from '@/state/use-built-store';
import { useLabStore } from '@/state/use-lab-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useLibraryStore } from '@/state/use-library-store';
import { clearMocks, openMockLibrary, openMockWorkspace } from '@/test/mock-ipc';
import { fakeBoardEnv, mockAsset } from '../../fakeBoardEnv';
import CardMenu from './CardMenu';

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, 'batch');
});

afterEach(() => {
  clearMocks();
  useLabStore.setState({ objects: null });
  useAppStore.setState({ notice: null });
  useProvidersStore.setState({ project: null, jobs: [] });
});

describe('CardMenu', () => {
  it('groups the actions and moves the card from the Move to submenu', async () => {
    const board = fakeBoardEnv();
    const asset = mockAsset('Character');
    render(<CardMenu asset={asset} waiting={undefined} refPicture={null} board={board} />);
    const menu = screen.getByRole('menu', { name: 'Character actions' });
    const labels = within(menu)
      .getAllByRole('menuitem')
      .map(i => i.textContent);
    expect(labels).toEqual([
      'Send to 3D',
      'Rename...',
      'Real size and fix-ups...',
      'Copy to project...',
      'Add to library...',
      'Open in Blender',
      'Move to',
    ]);
    expect(within(menu).getAllByRole('separator')).toHaveLength(2);
    expect(within(menu).queryByRole('menuitem', { name: 'Settings card' })).toBeNull();
    await userEvent.hover(within(menu).getByRole('menuitem', { name: 'Move to' }));
    const move = screen.getByRole('menu', { name: 'Move to' });
    expect(within(move).queryByRole('menuitem', { name: 'Blender' })).toBeNull();
    await userEvent.click(within(move).getByRole('menuitem', { name: 'Web' }));
    expect(board.actions.move).toHaveBeenCalledWith(asset, 'web');
  });

  it('hides the library and Blender before there is a model', () => {
    const asset = { ...mockAsset('Character'), versions: [] };
    render(<CardMenu asset={asset} waiting={undefined} refPicture={null} board={fakeBoardEnv()} />);
    expect(screen.queryByRole('menuitem', { name: 'Add to library...' })).toBeNull();
    expect(screen.queryByRole('menuitem', { name: 'Open in Blender' })).toBeNull();
  });

  it('leads with the By hand actions while the card waits for its model', async () => {
    const user = userEvent.setup();
    await useProvidersStore.getState().load('moon-island');
    const openHandoff = vi.fn(async () => {});
    const revealImage = vi.fn(async () => {});
    useProvidersStore.setState({ openHandoff, revealImage });
    const waiting = useProvidersStore.getState().jobs.find(j => j.id === 'lanterns-1')!;
    const asset = mockAsset('Stone lanterns');
    render(<CardMenu asset={asset} waiting={waiting} refPicture={null} board={fakeBoardEnv()} />);
    const labels = screen.getAllByRole('menuitem').map(i => i.textContent);
    expect(labels.slice(0, 5)).toEqual([
      'Copy export name',
      'Reveal image in Finder',
      'Open handoff folder',
      'Settings card',
      'Add model...',
    ]);
    await user.click(screen.getByRole('menuitem', { name: 'Reveal image in Finder' }));
    expect(revealImage).toHaveBeenCalledWith('lanterns-1', 'front.png');
    await user.click(screen.getByRole('menuitem', { name: 'Open handoff folder' }));
    expect(openHandoff).toHaveBeenCalledWith('lanterns-1');
    await user.click(screen.getByRole('menuitem', { name: 'Copy export name' }));
    expect(await navigator.clipboard.readText()).toBe(waiting.card!.exportName);
    expect(useAppStore.getState().notice).toMatchObject({
      title: 'Export name copied',
      detail: waiting.card!.exportName,
    });
  });

  it('says Show in Explorer on Windows', async () => {
    await useProvidersStore.getState().load('moon-island');
    const boot = useAppStore.getState().boot;
    useAppStore.setState({ boot: boot && { ...boot, os: 'windows' } });
    const waiting = useProvidersStore.getState().jobs.find(j => j.id === 'lanterns-1')!;
    render(
      <CardMenu
        asset={mockAsset('Stone lanterns')}
        waiting={waiting}
        refPicture={null}
        board={fakeBoardEnv()}
      />
    );
    expect(screen.getByRole('menuitem', { name: 'Show image in Explorer' })).toBeInTheDocument();
  });

  it('opens Rename, Real size and Send to 3D for its card', async () => {
    const openPanel = vi.fn(async () => {});
    useProvidersStore.setState({ openPanel });
    const board = fakeBoardEnv();
    const asset = mockAsset('Character');
    render(<CardMenu asset={asset} waiting={undefined} refPicture={null} board={board} />);
    await userEvent.click(screen.getByRole('menuitem', { name: 'Rename...' }));
    expect(board.actions.setRenaming).toHaveBeenCalledWith(asset);
    await userEvent.click(screen.getByRole('menuitem', { name: 'Real size and fix-ups...' }));
    expect(board.actions.setSizing).toHaveBeenCalledWith(asset);
    await userEvent.click(screen.getByRole('menuitem', { name: 'Send to 3D' }));
    expect(openPanel).toHaveBeenCalledWith('moon-island', 'character');
  });

  it('shows the other projects once Copy to project is picked', async () => {
    const asset = mockAsset('Character');
    const board = fakeBoardEnv();
    const copying = fakeBoardEnv({ actions: { ...board.actions, copyFor: 'character' } });
    render(<CardMenu asset={asset} waiting={undefined} refPicture={null} board={copying} />);
    const menu = screen.getByRole('menu', { name: 'Copy Character to project' });
    expect(menu).toHaveTextContent('Refs, versions and sends go along. It stays here too.');
    await userEvent.click(within(menu).getByRole('menuitem', { name: 'Rooftop cinema' }));
    expect(board.actions.copyTo).toHaveBeenCalledWith(
      asset,
      expect.objectContaining({ name: 'Rooftop cinema' })
    );
  });

  it('makes object refs from a ref picture', async () => {
    const asset = mockAsset('Desk lamp');
    render(
      <CardMenu
        asset={asset}
        waiting={undefined}
        refPicture="refs/desk_lamp.png"
        board={fakeBoardEnv()}
      />
    );
    await userEvent.click(screen.getByRole('menuitem', { name: 'Make object refs...' }));
    expect(useLabStore.getState().objects).toMatchObject({
      project: 'moon-island',
      source: 'refs/desk_lamp.png',
      sourceName: 'Desk lamp',
    });
  });

  it('rebuilds a built asset instead of sending it to 3D', async () => {
    const rebuild = vi.fn(async () => {});
    useBuiltStore.setState({ rebuild });
    const board = fakeBoardEnv();
    const asset = mockAsset('Character');
    const built: BuiltAsset = {
      asset: 'character',
      name: 'Character',
      source: { script: 'built/character.py', helpers: [], root: null, last: null },
      state: 'new',
      missing: [],
    };
    render(
      <CardMenu asset={asset} waiting={undefined} refPicture={null} built={built} board={board} />
    );
    expect(screen.queryByRole('menuitem', { name: 'Send to 3D' })).toBeNull();
    await userEvent.click(screen.getByRole('menuitem', { name: 'Rebuild' }));
    expect(rebuild).toHaveBeenCalledWith('moon-island', 'character', true);
    expect(board.actions.setMenuFor).toHaveBeenCalledWith(null);
  });

  it('adds the asset to the library, and offers the update waiting there', async () => {
    await openMockLibrary('demo');
    await useLibraryStore.getState().load();
    const asset = mockAsset('Character');
    render(<CardMenu asset={asset} waiting={undefined} refPicture={null} board={fakeBoardEnv()} />);
    await userEvent.click(screen.getByRole('menuitem', { name: 'Update to library v3...' }));
    expect(useLibraryStore.getState().updating).toEqual({
      project: 'moon-island',
      asset: 'character',
    });
    await userEvent.click(screen.getByRole('menuitem', { name: 'Add to library...' }));
    expect(useLibraryStore.getState().adding).toEqual({
      project: 'moon-island',
      asset: 'character',
    });
    useLibraryStore.setState({ view: null, adding: null, updating: null, plan: null });
  });
});
