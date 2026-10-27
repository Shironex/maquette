import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { HandoffChecklist } from '../HandoffChecklist';
import { DownloadsMatches } from '.';

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

describe('Check Downloads', () => {
  it('offers new Downloads files and takes one only when confirmed', async () => {
    const sheet = await openChecklist();
    await userEvent.click(within(sheet).getByRole('button', { name: 'Check Downloads' }));
    const dialog = await screen.findByRole('dialog', { name: 'New models in Downloads' });
    await within(dialog).findByText('model (3).glb');
    expect(within(dialog).getByText('~/Downloads', { exact: false })).toBeInTheDocument();
    const rows = within(dialog).getAllByRole('listitem');
    expect(rows).toHaveLength(3);
    // Nothing is taken by looking.
    expect(state('sigil-door-1')).toBe('waiting');
    const stranger = rows[2]!;
    expect(within(stranger).getByRole('button', { name: 'Use' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );

    const first = rows[0]!;
    const match = within(first).getByRole('combobox');
    const job = (match as HTMLSelectElement).value;
    await userEvent.click(within(first).getByRole('button', { name: 'Use' }));
    await waitFor(() => expect(state(job)).toBe('inbox'));
    expect(within(first).getByText(/^Copied for/)).toBeInTheDocument();
  });

  it('closes on Done', async () => {
    const sheet = await openChecklist();
    await userEvent.click(within(sheet).getByRole('button', { name: 'Check Downloads' }));
    const dialog = await screen.findByRole('dialog', { name: 'New models in Downloads' });
    await userEvent.click(within(dialog).getByRole('button', { name: 'Done' }));
    await waitFor(() =>
      expect(screen.queryByRole('dialog', { name: 'New models in Downloads' })).toBeNull()
    );
  });
});
