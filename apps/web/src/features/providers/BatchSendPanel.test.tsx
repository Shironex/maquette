import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useProvidersStore } from '@/state/providers-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { BatchSendPanel } from './BatchSendPanel';
import { HandoffChecklist } from './HandoffChecklist';

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({
    project: null,
    jobs: [],
    batch: null,
    checklistOpen: false,
    downloads: null,
    sending: false,
  });
});

async function openBatch(assets: string[]) {
  await openMockWorkspace('configured', undefined, 'batch');
  await useProvidersStore.getState().load('moon-island');
  render(
    <>
      <BatchSendPanel />
      <HandoffChecklist />
    </>
  );
  await useProvidersStore.getState().openBatch('moon-island', assets);
  return screen.findByRole('dialog', { name: `Send ${assets.length} assets to 3D` });
}

describe('the batch send panel', () => {
  it('writes one handoff folder per asset by hand, with a kind each, and opens the checklist', async () => {
    const panel = await openBatch(['desk-lamp', 'brush-cup']);
    expect(within(panel).getByText('Desk lamp')).toBeInTheDocument();
    expect(within(panel).getByText('1 image: brush_cup.png')).toBeInTheDocument();
    const kind = within(panel).getByRole('group', { name: 'Brush cup kind' });
    await userEvent.click(within(kind).getByRole('button', { name: 'Character' }));
    expect(
      screen.getByText('2 models · 1 prop, 1 character · By hand, Tripo settings')
    ).toBeInTheDocument();

    await userEvent.click(within(panel).getByRole('button', { name: 'Write 2 handoff folders' }));
    await screen.findByRole('dialog', { name: 'By hand checklist' });
    const fresh = useProvidersStore
      .getState()
      .jobs.filter(j => ['desk-lamp-2', 'brush-cup-2'].includes(j.id));
    expect(fresh.map(j => [j.assetName, j.variant, j.state])).toEqual([
      ['Desk lamp', 'prop', 'waiting'],
      ['Brush cup', 'character', 'waiting'],
    ]);
    expect(new Set(fresh.map(j => j.batch)).size).toBe(1);
    expect(fresh.every(j => j.handoff?.endsWith(`/handoff/${j.id}`))).toBe(true);
    expect(screen.queryByRole('dialog', { name: /assets to 3D/ })).toBeNull();
  });

  it('asks once before an API batch spends credits', async () => {
    const panel = await openBatch(['desk-lamp', 'calipers']);
    await userEvent.click(
      within(within(panel).getByRole('group', { name: 'Provider' })).getByRole('button', {
        name: 'Tripo',
      })
    );
    await userEvent.click(within(panel).getByRole('button', { name: 'Send 2 to Tripo' }));
    expect(within(panel).getByRole('alert')).toHaveTextContent('Send 2 assets to Tripo?');
    expect(useProvidersStore.getState().jobs.some(j => j.id === 'desk-lamp-2')).toBe(false);

    await userEvent.click(within(panel).getByRole('button', { name: 'Yes, send 2' }));
    await waitFor(() => expect(useProvidersStore.getState().batch).toBeNull());
    const fresh = useProvidersStore
      .getState()
      .jobs.filter(j => ['desk-lamp-2', 'calipers-2'].includes(j.id));
    expect(fresh.map(j => [j.provider, j.state])).toEqual([
      ['tripo', 'queued'],
      ['tripo', 'queued'],
    ]);
  });

  it('will not send an asset without refs', async () => {
    const panel = await openBatch(['desk-lamp', 'island']);
    expect(within(panel).getByText('No refs yet')).toBeInTheDocument();
    expect(within(panel).getByRole('button', { name: 'Write 2 handoff folders' })).toBeDisabled();
    expect(
      within(panel).getByText('Island has no refs yet. Send a pick to Refs first.')
    ).toBeInTheDocument();
  });
});
