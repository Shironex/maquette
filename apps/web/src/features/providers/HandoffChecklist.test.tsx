import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { DownloadsMatches } from './DownloadsMatches';
import { HandoffChecklist } from './HandoffChecklist';

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({
    project: null,
    jobs: [],
    checklistOpen: false,
    downloads: null,
    busy: [],
  });
});

async function openChecklist() {
  await openMockWorkspace('configured', undefined, 'batch');
  await useProvidersStore.getState().load('moon-island');
  render(
    <>
      <HandoffChecklist />
      <DownloadsMatches />
    </>
  );
  useProvidersStore.getState().setChecklistOpen(true);
  return screen.findByRole('dialog', { name: 'By hand checklist' });
}

const state = (id: string) => useProvidersStore.getState().jobs.find(j => j.id === id)?.state;

describe('the By hand checklist', () => {
  it('lists every handoff of the batch with its settings, folder and status', async () => {
    const sheet = await openChecklist();
    const batch = within(sheet).getByRole('region', { name: /^Batch batch-/ });
    expect(within(batch).getByText('2 of 6 back')).toBeInTheDocument();
    expect(within(batch).getAllByRole('listitem')).toHaveLength(6);
    expect(within(batch).getByText('In the Inbox')).toBeInTheDocument();
    expect(within(batch).getByText('Accepted as v1')).toBeInTheDocument();
    expect(within(batch).getAllByText(/^Tripo Image to 3D · Model H3\.1/).length).toBe(6);
    expect(
      within(sheet).getByRole('button', { name: 'Open the Desk lamp folder' })
    ).toBeInTheDocument();
    expect(within(sheet).getByText('5 models to make · nothing spent')).toBeInTheDocument();
  });

  it('adds a model from the file picker and the send leaves the waiting list', async () => {
    const sheet = await openChecklist();
    const lamp = within(sheet).getByText('Desk lamp').closest('li')!;
    await userEvent.click(within(lamp).getByRole('button', { name: 'Add model...' }));
    await waitFor(() => expect(state('desk-lamp-1')).toBe('inbox'));
    expect(within(lamp).queryByRole('button', { name: 'Add model...' })).toBeNull();
    expect(within(sheet).getByText('3 of 6 back')).toBeInTheDocument();
  });

  it('offers new Downloads files and takes one only when confirmed', async () => {
    const sheet = await openChecklist();
    await userEvent.click(within(sheet).getByRole('button', { name: 'Check Downloads' }));
    const dialog = await screen.findByRole('dialog', { name: 'New models in Downloads' });
    await within(dialog).findByText('model (3).glb');
    expect(within(dialog).getByText('~/Downloads', { exact: false })).toBeInTheDocument();
    const rows = within(dialog).getAllByRole('listitem');
    expect(rows).toHaveLength(3);
    // Nothing is taken by looking.
    expect(state('brush-cup-1')).toBe('waiting');
    const stranger = rows[2]!;
    expect(within(stranger).getByRole('button', { name: 'Use' })).toBeDisabled();

    const first = rows[0]!;
    const match = within(first).getByRole('combobox');
    const job = (match as HTMLSelectElement).value;
    await userEvent.click(within(first).getByRole('button', { name: 'Use' }));
    await waitFor(() => expect(state(job)).toBe('inbox'));
    expect(within(first).getByText(/^Copied for/)).toBeInTheDocument();
  });
});
