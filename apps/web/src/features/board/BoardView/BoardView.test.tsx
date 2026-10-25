import { act, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useLabStore } from '@/state/use-lab-store';
import { useProjectsStore } from '@/state/use-projects-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import BoardView from './BoardView';

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({
    project: null,
    jobs: [],
    batch: null,
    batchesSent: 0,
    busy: [],
  });
  useLabStore.setState({ objects: null });
});

async function renderBoard() {
  await openMockWorkspace('configured', undefined, 'batch');
  await useProvidersStore.getState().load('moon-island');
  render(<BoardView queue={() => null} />);
  await screen.findByRole('button', { name: /^Character, v2/ });
}

describe('the Board', () => {
  it('selects cards with their boxes and Shift, then sends the selection in one batch', async () => {
    await renderBoard();
    expect(screen.queryByRole('toolbar', { name: 'Selected assets' })).toBeNull();
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Turntable island' }));
    const user = userEvent.setup();
    await user.keyboard('{Shift>}');
    await user.click(screen.getByRole('checkbox', { name: 'Select Vending machine' }));
    await user.keyboard('{/Shift}');
    const bar = screen.getByRole('toolbar', { name: 'Selected assets' });
    expect(bar).toHaveTextContent('3 assets selected');
    expect(screen.getByRole('checkbox', { name: 'Select Character' })).toHaveAttribute(
      'aria-checked',
      'true'
    );
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Character' }));
    expect(bar).toHaveTextContent('2 assets selected');

    await userEvent.click(within(bar).getByRole('button', { name: 'Send to 3D' }));
    await waitFor(() =>
      expect(useProvidersStore.getState().batch?.draft?.items.map(i => i.name)).toEqual([
        'Turntable island',
        'Vending machine',
      ])
    );

    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('toolbar', { name: 'Selected assets' })).toBeNull();
  });

  it('clears the selection once the batch went out', async () => {
    await renderBoard();
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Desk lamp' }));
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Brush cup' }));
    const bar = screen.getByRole('toolbar', { name: 'Selected assets' });
    await userEvent.click(within(bar).getByRole('button', { name: 'Send to 3D' }));
    await waitFor(() => expect(useProvidersStore.getState().batch?.draft).toBeTruthy());
    await act(() => useProvidersStore.getState().sendBatch());
    expect(useProvidersStore.getState().batchesSent).toBe(1);
    await waitFor(() =>
      expect(screen.queryByRole('toolbar', { name: 'Selected assets' })).toBeNull()
    );
    expect(screen.getByRole('checkbox', { name: 'Select Desk lamp' })).toHaveAttribute(
      'aria-checked',
      'false'
    );
  });

  it('keeps the selection when the batch is only closed', async () => {
    await renderBoard();
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Desk lamp' }));
    await userEvent.click(
      within(screen.getByRole('toolbar', { name: 'Selected assets' })).getByRole('button', {
        name: 'Send to 3D',
      })
    );
    await waitFor(() => expect(useProvidersStore.getState().batch?.draft).toBeTruthy());
    await act(async () => useProvidersStore.getState().closeBatch());
    expect(screen.getByRole('toolbar', { name: 'Selected assets' })).toBeInTheDocument();
  });

  it('takes the model back from a waiting card', async () => {
    await renderBoard();
    const card = screen.getByRole('button', { name: 'Brush cup' }).closest('[data-drop-job]')!;
    expect(card).toHaveTextContent('Waiting for your model');
    await userEvent.click(
      within(card as HTMLElement).getByRole('button', { name: 'Add model...' })
    );
    await waitFor(() =>
      expect(useProvidersStore.getState().jobs.find(j => j.id === 'brush-cup-1')?.state).toBe(
        'inbox'
      )
    );
  });

  it('makes object refs from a Refs card picture', async () => {
    await renderBoard();
    await userEvent.click(screen.getByRole('button', { name: 'More for Desk lamp' }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Make object refs...' }));
    expect(useLabStore.getState().objects).toMatchObject({
      project: 'moon-island',
      source: 'refs/desk_lamp.png',
      sourceName: 'Desk lamp',
      aspect: '1:1',
      kind: 'prop',
    });
  });

  it('renames a card from its menu', async () => {
    await renderBoard();
    await userEvent.click(screen.getByRole('button', { name: 'More for Desk lamp' }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Rename...' }));
    const dialog = screen.getByRole('dialog', { name: 'Rename asset' });
    const field = within(dialog).getByRole('textbox', { name: 'Name' });
    expect(field).toHaveValue('Desk lamp');
    await userEvent.clear(field);
    await userEvent.type(field, 'Brass desk lamp');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Rename' }));
    await screen.findByRole('button', { name: 'More for Brass desk lamp' });
    expect(screen.queryByRole('dialog', { name: 'Rename asset' })).toBeNull();
  });

  it('opens the real size and fix-ups of a card from its menu', async () => {
    await renderBoard();
    await userEvent.click(screen.getByRole('button', { name: 'More for Character' }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Real size and fix-ups...' }));
    const dialog = screen.getByRole('dialog', { name: 'Real size and fix-ups' });
    expect(await within(dialog).findByText(/Measured 1.36 × 2 × 0.84 m/)).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
    expect(screen.queryByRole('dialog', { name: 'Real size and fix-ups' })).toBeNull();
  });

  it('keeps the rename dialog open with the reason when the name is taken', async () => {
    await renderBoard();
    await userEvent.click(screen.getByRole('button', { name: 'More for Desk lamp' }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Rename...' }));
    const dialog = screen.getByRole('dialog', { name: 'Rename asset' });
    const field = within(dialog).getByRole('textbox', { name: 'Name' });
    await userEvent.clear(field);
    await userEvent.type(field, 'brush cup');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Rename' }));
    expect(await within(dialog).findByText(/already called/)).toBeInTheDocument();
  });

  it('closes the rename dialog and the add form when another project opens', async () => {
    await renderBoard();
    await userEvent.click(screen.getByRole('button', { name: 'Add asset' }));
    expect(screen.getByRole('form', { name: 'Add asset' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'More for Desk lamp' }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Rename...' }));
    expect(screen.getByRole('dialog', { name: 'Rename asset' })).toBeInTheDocument();
    // Cmd+P works over the dialog.
    await act(() => useProjectsStore.getState().switchTo('rooftop-cinema'));
    expect(screen.queryByRole('dialog', { name: 'Rename asset' })).toBeNull();
    expect(screen.queryByRole('form', { name: 'Add asset' })).toBeNull();
  });

  it('copies a card to another project and keeps it here', async () => {
    await renderBoard();
    await userEvent.click(screen.getByRole('button', { name: 'More for Character' }));
    await userEvent.click(await screen.findByRole('menuitem', { name: /Copy to project/ }));
    const menu = await screen.findByRole('menu', { name: 'Copy Character to project' });
    await userEvent.click(within(menu).getByRole('menuitem', { name: 'Rooftop cinema' }));
    await waitFor(() =>
      expect(useAppStore.getState().notice?.title).toBe('Copied Character to Rooftop cinema')
    );
    expect(screen.queryByRole('menu', { name: 'Copy Character to project' })).toBeNull();
    expect(screen.getByRole('button', { name: /^Character, v2/ })).toBeInTheDocument();
  });
});
