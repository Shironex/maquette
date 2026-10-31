import { act, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useLabStore } from '@/state/use-lab-store';
import { useProjectsStore } from '@/state/use-projects-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useTrashStore } from '@/state/use-trash-store';
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
  useTrashStore.setState({ project: null, entries: [], restoring: null });
});

async function renderBoard() {
  await openMockWorkspace('configured', undefined, 'batch');
  await useProvidersStore.getState().load('frostveil');
  render(<BoardView />);
  await screen.findByRole('button', { name: /^Elsheria, v2/ });
}

describe('the Board', () => {
  it('selects cards with their boxes and Shift, then sends the selection in one batch', async () => {
    await renderBoard();
    expect(screen.queryByRole('toolbar', { name: 'Selected assets' })).toBeNull();
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Orb shrine' }));
    const user = userEvent.setup();
    await user.keyboard('{Shift>}');
    await user.click(screen.getByRole('checkbox', { name: 'Select Crystal lantern' }));
    await user.keyboard('{/Shift}');
    const bar = screen.getByRole('toolbar', { name: 'Selected assets' });
    expect(bar).toHaveTextContent('3 assets selected');
    expect(screen.getByRole('checkbox', { name: 'Select Elsheria' })).toHaveAttribute(
      'aria-checked',
      'true'
    );
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Elsheria' }));
    expect(bar).toHaveTextContent('2 assets selected');

    await userEvent.click(within(bar).getByRole('button', { name: 'Send to 3D' }));
    await waitFor(() =>
      expect(useProvidersStore.getState().batch?.draft?.items.map(i => i.name)).toEqual([
        'Orb shrine',
        'Crystal lantern',
      ])
    );

    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('toolbar', { name: 'Selected assets' })).toBeNull();
  });

  it('clears the selection once the batch went out', async () => {
    await renderBoard();
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Lantern post' }));
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Sigil door' }));
    const bar = screen.getByRole('toolbar', { name: 'Selected assets' });
    await userEvent.click(within(bar).getByRole('button', { name: 'Send to 3D' }));
    await waitFor(() => expect(useProvidersStore.getState().batch?.draft).toBeTruthy());
    await act(() => useProvidersStore.getState().sendBatch());
    expect(useProvidersStore.getState().batchesSent).toBe(1);
    await waitFor(() =>
      expect(screen.queryByRole('toolbar', { name: 'Selected assets' })).toBeNull()
    );
    expect(screen.getByRole('checkbox', { name: 'Select Lantern post' })).toHaveAttribute(
      'aria-checked',
      'false'
    );
  });

  it('keeps the selection when the batch is only closed', async () => {
    await renderBoard();
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Lantern post' }));
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
    const card = screen.getByRole('button', { name: 'Sigil door' }).closest('[data-drop-job]')!;
    expect(card).toHaveTextContent('Waiting for your model');
    await userEvent.click(
      within(card as HTMLElement).getByRole('button', { name: 'Add model...' })
    );
    await waitFor(() =>
      expect(useProvidersStore.getState().jobs.find(j => j.id === 'sigil-door-1')?.state).toBe(
        'inbox'
      )
    );
  });

  it('makes object refs from a Refs card picture', async () => {
    await renderBoard();
    await userEvent.click(screen.getByRole('button', { name: 'More for Lantern post' }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Make object refs...' }));
    expect(useLabStore.getState().objects).toMatchObject({
      project: 'frostveil',
      source: 'refs/lantern_post.png',
      sourceName: 'Lantern post',
      aspect: '1:1',
      kind: 'prop',
    });
  });

  it('renames a card from its menu', async () => {
    await renderBoard();
    await userEvent.click(screen.getByRole('button', { name: 'More for Lantern post' }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Rename...' }));
    const dialog = screen.getByRole('dialog', { name: 'Rename asset' });
    const field = within(dialog).getByRole('textbox', { name: 'Name' });
    expect(field).toHaveValue('Lantern post');
    await userEvent.clear(field);
    await userEvent.type(field, 'Brass lantern post');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Rename' }));
    await screen.findByRole('button', { name: 'More for Brass lantern post' });
    expect(screen.queryByRole('dialog', { name: 'Rename asset' })).toBeNull();
  });

  it('opens the real size and fix-ups of a card from its menu', async () => {
    await renderBoard();
    await userEvent.click(screen.getByRole('button', { name: 'More for Elsheria' }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Real size and fix-ups...' }));
    const dialog = screen.getByRole('dialog', { name: 'Real size and fix-ups' });
    expect(await within(dialog).findByText(/Measured 0.94 × 1.7 × 0.44 m/)).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
    expect(screen.queryByRole('dialog', { name: 'Real size and fix-ups' })).toBeNull();
  });

  it('keeps the rename dialog open with the reason when the name is taken', async () => {
    await renderBoard();
    await userEvent.click(screen.getByRole('button', { name: 'More for Lantern post' }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Rename...' }));
    const dialog = screen.getByRole('dialog', { name: 'Rename asset' });
    const field = within(dialog).getByRole('textbox', { name: 'Name' });
    await userEvent.clear(field);
    await userEvent.type(field, 'sigil door');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Rename' }));
    expect(await within(dialog).findByText(/already called/)).toBeInTheDocument();
  });

  it('closes the rename dialog and the add form when another project opens', async () => {
    await renderBoard();
    await userEvent.click(screen.getByRole('button', { name: 'Add asset' }));
    expect(screen.getByRole('form', { name: 'Add asset' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'More for Lantern post' }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Rename...' }));
    expect(screen.getByRole('dialog', { name: 'Rename asset' })).toBeInTheDocument();
    // Cmd+P works over the dialog.
    await act(() => useProjectsStore.getState().switchTo('frost-cathedral'));
    expect(screen.queryByRole('dialog', { name: 'Rename asset' })).toBeNull();
    expect(screen.queryByRole('form', { name: 'Add asset' })).toBeNull();
  });

  it('copies a card to another project and keeps it here', async () => {
    await renderBoard();
    await userEvent.click(screen.getByRole('button', { name: 'More for Elsheria' }));
    await userEvent.click(await screen.findByRole('menuitem', { name: /Copy to project/ }));
    const menu = await screen.findByRole('menu', { name: 'Copy Elsheria to project' });
    await userEvent.click(within(menu).getByRole('menuitem', { name: 'Frost cathedral' }));
    await waitFor(() =>
      expect(useAppStore.getState().notice?.title).toBe('Copied Elsheria to Frost cathedral')
    );
    expect(screen.queryByRole('menu', { name: 'Copy Elsheria to project' })).toBeNull();
    expect(screen.getByRole('button', { name: /^Elsheria, v2/ })).toBeInTheDocument();
  });

  it('removes a card from its menu and the selection from the bar', async () => {
    await renderBoard();
    await userEvent.click(screen.getByRole('button', { name: 'More for Lantern post' }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Remove from project...' }));
    const dialog = screen.getByRole('dialog', { name: 'Remove Lantern post from the project?' });
    await userEvent.click(within(dialog).getByRole('button', { name: 'Remove' }));
    await waitFor(() =>
      expect(screen.queryByRole('button', { name: 'More for Lantern post' })).toBeNull()
    );
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(useAppStore.getState().notice?.title).toBe('Removed Lantern post');

    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Orb shrine' }));
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Elsheria' }));
    const bar = screen.getByRole('toolbar', { name: 'Selected assets' });
    await userEvent.click(within(bar).getByRole('button', { name: 'Remove...' }));
    const many = screen.getByRole('dialog', { name: 'Remove 2 assets from the project?' });
    await userEvent.click(within(many).getByRole('button', { name: 'Remove 2 assets' }));
    await waitFor(() =>
      expect(screen.queryByRole('toolbar', { name: 'Selected assets' })).toBeNull()
    );
    expect(screen.queryByRole('button', { name: /^Elsheria, v2/ })).toBeNull();
    expect(await screen.findByRole('button', { name: /Removed/ })).toHaveTextContent('3');
  });
});
