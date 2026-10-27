import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useDownloadsStore } from '@/state/use-downloads-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { ArrivalPicker } from '.';

afterEach(() => {
  clearMocks();
  useDownloadsStore.setState({ status: null, pickerOpen: false, choices: {}, busy: [] });
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
  return screen.findByRole('dialog', { name: 'Which asset is this?' });
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

  it('adds a download to the send picked for it', async () => {
    const dialog = await openPicker();
    expect(within(dialog).getByText('model (3).glb')).toBeInTheDocument();
    const add = within(dialog).getByRole('button', { name: 'Add to inbox' });
    expect(add).toHaveAttribute('aria-disabled', 'true');

    const select = within(dialog).getByRole('combobox', { name: 'Asset for model (3).glb' });
    const job = useDownloadsStore.getState().status?.pending[0]?.job ?? '';
    await userEvent.selectOptions(select, job);
    await userEvent.click(within(dialog).getByRole('button', { name: 'Add to inbox' }));

    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(useDownloadsStore.getState().status?.arrivals[0]?.state).toBe('imported');
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
