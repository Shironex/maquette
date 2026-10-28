import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useDownloadsStore } from '@/state/use-downloads-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { ArrivalPicker } from '.';

afterEach(() => {
  clearMocks();
  useDownloadsStore.setState({
    status: null,
    pickerOpen: false,
    chooserFor: null,
    previews: {},
    busy: [],
  });
  useProvidersStore.setState({ project: null, jobs: [] });
});

/** Opens the picker over one download ("model (3).glb") that asks which asset it is. */
async function openPicker() {
  await openMockWorkspace('configured', undefined, 'batch', undefined, 'unmatched');
  await waitFor(async () => {
    await useDownloadsStore.getState().load();
    expect(useDownloadsStore.getState().status?.arrivals).toHaveLength(1);
  });
  render(<ArrivalPicker />);
  useDownloadsStore.getState().openPicker();
  return screen.findByRole('dialog', { name: 'New downloads' });
}

const jobState = async (id: string) => {
  await useProvidersStore.getState().load('frostveil');
  return useProvidersStore.getState().jobs.find(j => j.id === id)?.state;
};

describe('Which asset is this?', () => {
  it('stays closed until asked', async () => {
    await openMockWorkspace('configured', undefined, 'batch');
    render(<ArrivalPicker />);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('sends a download to the Inbox for the asset picked for it', async () => {
    const dialog = await openPicker();
    expect(within(dialog).getByText('model (3).glb')).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole('button', { name: 'Attach to an asset' }));
    const chooser = await screen.findByRole('dialog', { name: 'Attach to an asset' });
    const job = useDownloadsStore.getState().status?.pending[0]?.job ?? '';
    await userEvent.keyboard('{Enter}');

    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(chooser).not.toBeInTheDocument();
    expect(useDownloadsStore.getState().status?.arrivals[0]?.state).toBe('attached');
    // Like a named export: in the Inbox for the send, which accepting it closes.
    expect(await jobState(job)).toBe('inbox');
  });

  it('leaves a file that is not a Maquette model alone', async () => {
    const dialog = await openPicker();
    await userEvent.click(within(dialog).getByRole('button', { name: 'Not a Maquette model' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(useDownloadsStore.getState().status?.arrivals[0]?.state).toBe('dismissed');
  });

  it('closes on Done and keeps the question for later', async () => {
    const dialog = await openPicker();
    await userEvent.click(within(dialog).getByRole('button', { name: 'Done' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(useDownloadsStore.getState().status?.arrivals[0]?.state).toBe('unmatched');
  });
});
