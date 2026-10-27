import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { HandoffChecklist } from '.';

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
  await useProvidersStore.getState().load('frostveil');
  render(<HandoffChecklist />);
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
    expect(
      within(batch).getAllByText(/^Tripo Image to 3D · Tab HD Model · AI Model H3\.1/).length
    ).toBe(6);
    expect(
      within(sheet).getByRole('button', { name: 'Open the Lantern post folder' })
    ).toBeInTheDocument();
    expect(within(sheet).getByText('5 models to make · no API credits')).toBeInTheDocument();
  });

  it('adds a model from the file picker and the send leaves the waiting list', async () => {
    const sheet = await openChecklist();
    const lamp = within(sheet).getByText('Lantern post').closest('li')!;
    await userEvent.click(within(lamp).getByRole('button', { name: 'Add model...' }));
    await waitFor(() => expect(state('lantern-post-1')).toBe('inbox'));
    expect(within(lamp).queryByRole('button', { name: 'Add model...' })).toBeNull();
    expect(within(sheet).getByText('3 of 6 back')).toBeInTheDocument();
  });
});
