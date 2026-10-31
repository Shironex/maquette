import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { BuiltAsset } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { useBuiltStore } from '@/state/use-built-store';
import { useLabStore } from '@/state/use-lab-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useRefsStore } from '@/state/use-refs-store';
import { useLibraryStore } from '@/state/use-library-store';
import { useQuickViewStore } from '@/state/use-quick-view-store';
import { clearMocks, openMockLibrary, openMockWorkspace } from '@/test/mock-ipc';
import { fakeBoardEnv, mockAsset } from '../../fakeBoardEnv';
import { QuickView } from '@/features/viewer/QuickView';
import CardMenu from './CardMenu';

vi.mock('@/features/viewer/ModelPreview', () => ({ default: () => null }));

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
    const asset = mockAsset('Elsheria');
    render(<CardMenu asset={asset} waiting={undefined} refPicture={null} board={board} />);
    const menu = screen.getByRole('menu', { name: 'Elsheria actions' });
    const labels = within(menu)
      .getAllByRole('menuitem')
      .map(i => i.textContent);
    expect(labels).toEqual([
      'Send to 3D',
      'Refs and views...',
      'Rename...',
      'Real size and fix-ups...',
      'Copy to project...',
      'Add a model file as the next version...',
      'Add to library...',
      'Inspect 3D model',
      'Fix in rig doctor',
      'Open in Blender',
      'Remove from project...',
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

  it('opens the refs sheet of the asset', async () => {
    const asset = mockAsset('Elsheria');
    render(<CardMenu asset={asset} waiting={undefined} refPicture={null} board={fakeBoardEnv()} />);
    await userEvent.click(screen.getByRole('menuitem', { name: 'Refs and views...' }));
    expect(useRefsStore.getState().sheet).toMatchObject({ asset: asset.id, mode: 'refs' });
    useRefsStore.setState({ sheet: null, view: null });
  });

  it('hides the library, the 3D quick view and Blender before there is a model', () => {
    const asset = { ...mockAsset('Elsheria'), versions: [] };
    render(<CardMenu asset={asset} waiting={undefined} refPicture={null} board={fakeBoardEnv()} />);
    expect(screen.queryByRole('menuitem', { name: 'Add to library...' })).toBeNull();
    expect(screen.queryByRole('menuitem', { name: 'Inspect 3D model' })).toBeNull();
    expect(screen.queryByRole('menuitem', { name: 'Open in Blender' })).toBeNull();
  });

  it('opens the newest version in the 3D quick view, which Escape closes', async () => {
    const asset = mockAsset('Elsheria');
    const { rerender } = render(
      <>
        <CardMenu asset={asset} waiting={undefined} refPicture={null} board={fakeBoardEnv()} />
        <QuickView />
      </>
    );
    await userEvent.click(screen.getByRole('menuitem', { name: 'Inspect 3D model' }));
    // The Board closes its menu on a pick; Escape waits for an open menu to go first.
    rerender(<QuickView />);
    const dialog = screen.getByRole('dialog', { name: 'Elsheria' });
    expect(dialog).toHaveAccessibleDescription(/^v2 · 115.7k triangles · 1.1 MB · Frostveil$/);
    expect(within(dialog).getByRole('button', { name: 'Bones' })).toBeInTheDocument();
    expect(useQuickViewStore.getState().target).toMatchObject({
      name: 'Elsheria',
      version: 2,
      file: expect.stringMatching(/\/projects\/frostveil\/models\/elsheria_v2\.glb$/),
      inspect: { project: 'frostveil', asset: 'elsheria', version: 2 },
    });
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('leads with the By hand actions while the card waits for its model', async () => {
    const user = userEvent.setup();
    await useProvidersStore.getState().load('frostveil');
    const openHandoff = vi.fn(async () => {});
    const revealImage = vi.fn(async () => {});
    useProvidersStore.setState({ openHandoff, revealImage });
    const waiting = useProvidersStore.getState().jobs.find(j => j.id === 'guardian-statues-1')!;
    const asset = mockAsset('Guardian statues');
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
    expect(revealImage).toHaveBeenCalledWith('guardian-statues-1', 'front.png');
    await user.click(screen.getByRole('menuitem', { name: 'Open handoff folder' }));
    expect(openHandoff).toHaveBeenCalledWith('guardian-statues-1');
    await user.click(screen.getByRole('menuitem', { name: 'Copy export name' }));
    expect(await navigator.clipboard.readText()).toBe(waiting.card!.exportName);
    expect(useAppStore.getState().notice).toMatchObject({
      title: 'Export name copied',
      detail: waiting.card!.exportName,
    });
  });

  it('says Show in Explorer on Windows', async () => {
    await useProvidersStore.getState().load('frostveil');
    const boot = useAppStore.getState().boot;
    useAppStore.setState({ boot: boot && { ...boot, os: 'windows' } });
    const waiting = useProvidersStore.getState().jobs.find(j => j.id === 'guardian-statues-1')!;
    render(
      <CardMenu
        asset={mockAsset('Guardian statues')}
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
    const asset = mockAsset('Elsheria');
    render(<CardMenu asset={asset} waiting={undefined} refPicture={null} board={board} />);
    await userEvent.click(screen.getByRole('menuitem', { name: 'Rename...' }));
    expect(board.actions.setRenaming).toHaveBeenCalledWith(asset);
    await userEvent.click(screen.getByRole('menuitem', { name: 'Real size and fix-ups...' }));
    expect(board.actions.setSizing).toHaveBeenCalledWith(asset);
    await userEvent.click(screen.getByRole('menuitem', { name: 'Send to 3D' }));
    expect(openPanel).toHaveBeenCalledWith('frostveil', 'elsheria');
  });

  it('asks before removing the card, in the danger colour, last of its section', async () => {
    const board = fakeBoardEnv();
    const asset = mockAsset('Elsheria');
    render(<CardMenu asset={asset} waiting={undefined} refPicture={null} board={board} />);
    const remove = screen.getByRole('menuitem', { name: 'Remove from project...' });
    expect(remove).toHaveClass('text-danger');
    await userEvent.hover(remove);
    expect(
      await screen.findByText(
        'Takes the card off the Board and moves its files to the workspace trash, where Recently removed can restore it'
      )
    ).toBeInTheDocument();
    await userEvent.click(remove);
    expect(board.actions.setRemoving).toHaveBeenCalledWith([asset]);
  });

  it('shows the other projects once Copy to project is picked', async () => {
    const asset = mockAsset('Elsheria');
    const board = fakeBoardEnv();
    const copying = fakeBoardEnv({ actions: { ...board.actions, copyFor: 'elsheria' } });
    render(<CardMenu asset={asset} waiting={undefined} refPicture={null} board={copying} />);
    const menu = screen.getByRole('menu', { name: 'Copy Elsheria to project' });
    expect(menu).toHaveTextContent('Refs, versions and sends go along. It stays here too.');
    await userEvent.click(within(menu).getByRole('menuitem', { name: 'Frost cathedral' }));
    expect(board.actions.copyTo).toHaveBeenCalledWith(
      asset,
      expect.objectContaining({ name: 'Frost cathedral' })
    );
  });

  it('makes object refs from a ref picture', async () => {
    const asset = mockAsset('Lantern post');
    render(
      <CardMenu
        asset={asset}
        waiting={undefined}
        refPicture="refs/lantern_post.png"
        board={fakeBoardEnv()}
      />
    );
    await userEvent.click(screen.getByRole('menuitem', { name: 'Make object refs...' }));
    expect(useLabStore.getState().objects).toMatchObject({
      project: 'frostveil',
      source: 'refs/lantern_post.png',
      sourceName: 'Lantern post',
    });
  });

  it('rebuilds a built asset instead of sending it to 3D', async () => {
    const rebuild = vi.fn(async () => {});
    useBuiltStore.setState({ rebuild });
    const board = fakeBoardEnv();
    const asset = mockAsset('Elsheria');
    const built: BuiltAsset = {
      asset: 'elsheria',
      name: 'Elsheria',
      source: { script: 'built/elsheria.py', helpers: [], root: null, last: null },
      state: 'new',
      missing: [],
    };
    render(
      <CardMenu asset={asset} waiting={undefined} refPicture={null} built={built} board={board} />
    );
    expect(screen.queryByRole('menuitem', { name: 'Send to 3D' })).toBeNull();
    expect(
      screen.queryByRole('menuitem', { name: 'Add a model file as the next version...' })
    ).toBeNull();
    await userEvent.click(screen.getByRole('menuitem', { name: 'Rebuild' }));
    expect(rebuild).toHaveBeenCalledWith('frostveil', 'elsheria', true);
    expect(board.actions.setMenuFor).toHaveBeenCalledWith(null);
  });

  it('adds the asset to the library, and offers the update waiting there', async () => {
    await openMockLibrary('demo');
    await useLibraryStore.getState().load();
    const asset = mockAsset('Elsheria');
    render(<CardMenu asset={asset} waiting={undefined} refPicture={null} board={fakeBoardEnv()} />);
    await userEvent.click(screen.getByRole('menuitem', { name: 'Update to library v3...' }));
    expect(useLibraryStore.getState().updating).toEqual({
      project: 'frostveil',
      asset: 'elsheria',
    });
    await userEvent.click(screen.getByRole('menuitem', { name: 'Add to library...' }));
    expect(useLibraryStore.getState().adding).toEqual({
      project: 'frostveil',
      asset: 'elsheria',
    });
    useLibraryStore.setState({ view: null, adding: null, updating: null, plan: null });
  });

  it('copies a model file someone fixed into the Inbox for the card', async () => {
    const asset = { ...mockAsset('Elsheria'), versions: [] };
    render(<CardMenu asset={asset} waiting={undefined} refPicture={null} board={fakeBoardEnv()} />);
    await userEvent.click(
      screen.getByRole('menuitem', { name: 'Add a model file as the next version...' })
    );
    await vi.waitFor(() =>
      expect(useAppStore.getState().notice?.title).toBe('Elsheria is in the Inbox')
    );
  });
});
