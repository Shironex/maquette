import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useDownloadsStore } from '@/state/use-downloads-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { ArrivalPicker } from '..';

afterEach(() => {
  clearMocks();
  useDownloadsStore.setState({
    status: null,
    pickerOpen: false,
    chooserFor: null,
    previews: {},
    busy: [],
  });
});

/** The picker over the batch export, with the chooser open for its first unmatched file. */
async function openChooser() {
  await openMockWorkspace('configured', undefined, 'batch', undefined, 'look');
  await waitFor(async () => {
    await useDownloadsStore.getState().load();
    expect(useDownloadsStore.getState().status?.arrivals).toHaveLength(3);
  });
  render(<ArrivalPicker />);
  useDownloadsStore.getState().openPicker();
  const picker = await screen.findByRole('dialog', { name: 'New downloads' });
  const ask = within(picker).getByRole('region', { name: 'Which asset is this?' });
  await userEvent.click(within(ask).getAllByRole('button', { name: 'Attach to an asset' })[0]!);
  return screen.findByRole('dialog', { name: 'Attach to an asset' });
}

const names = (dialog: HTMLElement) =>
  within(dialog)
    .getAllByRole('option')
    .map(o => o.querySelector('.font-medium')?.textContent);

describe('the asset chooser', () => {
  it('lists the waiting sends first and narrows to a search', async () => {
    const chooser = await openChooser();
    expect(within(chooser).getByText('frost arch 3d model.glb')).toBeInTheDocument();
    const options = within(chooser).getAllByRole('option');
    expect(options[0]).toHaveTextContent(/Waiting for its Tripo model/);
    const search = within(chooser).getByRole('combobox');
    expect(search).toHaveFocus();
    await userEvent.type(search, 'sigil');
    expect(names(chooser)).toEqual(['Sigil door']);
    await userEvent.clear(search);
    await userEvent.type(search, 'nothing like it');
    expect(chooser).toHaveTextContent('No asset matches');
  });

  it('attaches with the keyboard and moves on to the next file', async () => {
    const chooser = await openChooser();
    const search = within(chooser).getByRole('combobox');
    await userEvent.type(search, 'step ring{Enter}');
    await waitFor(() =>
      expect(within(chooser).getByText('rune circle 3d model.glb')).toBeInTheDocument()
    );
    expect(search).toHaveValue('');
    expect(chooser).toHaveTextContent(/Enter sends it to the Inbox/);
    await userEvent.keyboard('{ArrowDown}{Enter}');
    await waitFor(() =>
      expect(screen.queryByRole('dialog', { name: 'Attach to an asset' })).toBeNull()
    );
    const attached = useDownloadsStore
      .getState()
      .status?.arrivals.filter(a => a.state === 'attached')
      .map(a => a.assetName);
    expect(attached).toHaveLength(2);
    expect(attached).toContain('Step ring');
    // The look is still there, so the picker stays.
    expect(screen.getByRole('dialog', { name: 'New downloads' })).toBeInTheDocument();
  });

  it('clears the search on Escape, then closes alone', async () => {
    const chooser = await openChooser();
    const search = within(chooser).getByRole('combobox');
    await userEvent.type(search, 'door');
    await userEvent.keyboard('{Escape}');
    expect(search).toHaveValue('');
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('dialog', { name: 'Attach to an asset' })).toBeNull();
    expect(screen.getByRole('dialog', { name: 'New downloads' })).toBeInTheDocument();
  });
});
