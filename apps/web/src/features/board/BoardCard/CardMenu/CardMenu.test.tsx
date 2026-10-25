import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useLabStore } from '@/state/use-lab-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { fakeBoardEnv, mockAsset } from '../../fakeBoardEnv';
import CardMenu from './CardMenu';

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, 'batch');
});

afterEach(() => {
  clearMocks();
  useLabStore.setState({ objects: null });
});

describe('CardMenu', () => {
  it('lists the actions and the columns the card can move to', async () => {
    const board = fakeBoardEnv();
    const asset = mockAsset('Character');
    render(<CardMenu asset={asset} waiting={undefined} refPicture={null} board={board} />);
    const menu = screen.getByRole('menu', { name: 'Character actions' });
    expect(within(menu).queryByRole('menuitem', { name: 'Move to Blender' })).toBeNull();
    expect(within(menu).queryByRole('menuitem', { name: 'Make object refs...' })).toBeNull();
    await userEvent.click(within(menu).getByRole('menuitem', { name: 'Move to Web' }));
    expect(board.actions.move).toHaveBeenCalledWith(asset, 'web');
  });

  it('opens Rename and Send to 3D for its card', async () => {
    const openPanel = vi.fn(async () => {});
    useProvidersStore.setState({ openPanel });
    const board = fakeBoardEnv();
    const asset = mockAsset('Character');
    render(<CardMenu asset={asset} waiting={undefined} refPicture={null} board={board} />);
    await userEvent.click(screen.getByRole('menuitem', { name: 'Rename...' }));
    expect(board.actions.setRenaming).toHaveBeenCalledWith(asset);
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
});
