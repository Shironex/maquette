import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Asset } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useTrashStore } from '@/state/use-trash-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { mockAsset } from '../fakeBoardEnv';
import RemoveAssetsDialog from './RemoveAssetsDialog';

beforeEach(async () => {
  await openMockWorkspace();
  await useProvidersStore.getState().load('frostveil');
});

afterEach(() => {
  clearMocks();
  useTrashStore.setState({ project: null, entries: [], restoring: null });
  useProvidersStore.setState({ project: null, jobs: [] });
  useAppStore.setState({ notice: null });
});

function renderDialog(assets: Asset[]) {
  const onDone = vi.fn();
  const onClose = vi.fn();
  render(
    <RemoveAssetsDialog
      project="frostveil"
      assets={assets}
      jobs={useProvidersStore.getState().jobs}
      onDone={onDone}
      onClose={onClose}
    />
  );
  return { onDone, onClose, dialog: screen.getByRole('dialog') };
}

describe('RemoveAssetsDialog', () => {
  it('names the asset, lists the send it stops and removes it', async () => {
    const { onDone, dialog } = renderDialog([mockAsset('Guardian statues')]);
    expect(dialog).toHaveAccessibleName('Remove Guardian statues from the project?');
    expect(dialog).toHaveTextContent('go to the workspace trash');
    expect(dialog).toHaveTextContent('Recently removed on the Board can restore it');
    const sends = screen.getByRole('list', { name: 'Sends that stop' });
    expect(sends).toHaveTextContent('Guardian statues: waiting for your model, made by hand');
    // The safe answer has the focus.
    expect(screen.getByRole('button', { name: 'Cancel' })).toHaveFocus();
    await userEvent.click(screen.getByRole('button', { name: 'Remove' }));
    await vi.waitFor(() => expect(onDone).toHaveBeenCalledOnce());
    const left = useAppStore.getState().workspace?.project?.assets ?? [];
    expect(left.some(a => a.id === 'guardian-statues')).toBe(false);
    expect(useAppStore.getState().notice?.action?.label).toBe('Undo');
  });

  it('counts a selection and keeps the asset a paid send is making', async () => {
    const { dialog } = renderDialog([mockAsset('Guardian statues'), mockAsset('Veil gate')]);
    expect(dialog).toHaveAccessibleName('Remove 2 assets from the project?');
    const names = within(screen.getByRole('list', { name: 'Assets to remove' }))
      .getAllByRole('listitem')
      .map(li => li.textContent);
    expect(names).toEqual(['Guardian statues', 'Veil gate']);
    expect(screen.getByRole('note')).toHaveTextContent('Tripo is making Veil gate now');
    expect(screen.getByRole('button', { name: 'Remove Guardian statues' })).toBeEnabled();
  });

  it('cannot remove an asset whose only send is being made', () => {
    renderDialog([mockAsset('Veil gate')]);
    expect(screen.getByRole('button', { name: 'Remove' })).toHaveAttribute('aria-disabled', 'true');
  });

  it('closes on Escape and on Cancel without a call', async () => {
    const { onClose, onDone } = renderDialog([mockAsset('Elsheria')]);
    await userEvent.keyboard('{Escape}');
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(onClose).toHaveBeenCalledTimes(2);
    expect(onDone).not.toHaveBeenCalled();
  });
});
