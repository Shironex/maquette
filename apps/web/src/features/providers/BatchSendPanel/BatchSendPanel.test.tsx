import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { BatchSendPanel } from '.';
import { HandoffChecklist } from '../HandoffChecklist';

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
  await useProvidersStore.getState().load('frostveil');
  render(
    <>
      <BatchSendPanel />
      <HandoffChecklist />
    </>
  );
  await useProvidersStore.getState().openBatch('frostveil', assets);
  return screen.findByRole('dialog', { name: `Send ${assets.length} assets to 3D` });
}

describe('the batch send panel', () => {
  it('writes one handoff folder per asset by hand, with a kind each, and opens the checklist', async () => {
    const panel = await openBatch(['lantern-post', 'sigil-door']);
    expect(within(panel).getByText('Lantern post')).toBeInTheDocument();
    expect(within(panel).getByText('1 image: sigil_door.png')).toBeInTheDocument();
    const kind = within(panel).getByRole('group', { name: 'Sigil door kind' });
    await userEvent.click(within(kind).getByRole('button', { name: 'Character' }));
    expect(
      screen.getByText('2 models · 1 prop, 1 character · By hand, Tripo settings')
    ).toBeInTheDocument();

    await userEvent.click(within(panel).getByRole('button', { name: 'Write 2 handoff folders' }));
    await screen.findByRole('dialog', { name: 'By hand checklist' });
    const fresh = useProvidersStore
      .getState()
      .jobs.filter(j => ['lantern-post-2', 'sigil-door-2'].includes(j.id));
    expect(fresh.map(j => [j.assetName, j.variant, j.state])).toEqual([
      ['Lantern post', 'prop', 'waiting'],
      ['Sigil door', 'character', 'waiting'],
    ]);
    expect(new Set(fresh.map(j => j.batch)).size).toBe(1);
    expect(fresh.every(j => j.handoff?.endsWith(`/handoff/${j.id}`))).toBe(true);
    expect(screen.queryByRole('dialog', { name: /assets to 3D/ })).toBeNull();
  });

  it('asks once before an API batch spends credits', async () => {
    const panel = await openBatch(['lantern-post', 'step-ring']);
    await userEvent.click(
      within(within(panel).getByRole('group', { name: 'Provider' })).getByRole('button', {
        name: 'Tripo',
      })
    );
    await userEvent.click(within(panel).getByRole('button', { name: 'Send 2 to Tripo' }));
    expect(within(panel).getByRole('alert')).toHaveTextContent('Send 2 assets to Tripo?');
    expect(useProvidersStore.getState().jobs.some(j => j.id === 'lantern-post-2')).toBe(false);

    await userEvent.click(within(panel).getByRole('button', { name: 'Yes, send 2' }));
    await waitFor(() => expect(useProvidersStore.getState().batch).toBeNull());
    const fresh = useProvidersStore
      .getState()
      .jobs.filter(j => ['lantern-post-2', 'step-ring-2'].includes(j.id));
    expect(fresh.map(j => [j.provider, j.state])).toEqual([
      ['tripo', 'queued'],
      ['tripo', 'queued'],
    ]);
  });

  it('will not send an asset without refs', async () => {
    const panel = await openBatch(['lantern-post', 'veil-platform']);
    expect(within(panel).getByText('No refs yet')).toBeInTheDocument();
    expect(within(panel).getByRole('button', { name: 'Write 2 handoff folders' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    expect(
      within(panel).getByText('Veil platform has no refs yet. Send a pick to Refs first.')
    ).toBeInTheDocument();
  });

  it('gives one asset its own settings and keeps the others on the batch', async () => {
    const panel = await openBatch(['lantern-post', 'step-ring', 'sigil-door']);
    await userEvent.click(
      within(within(panel).getByRole('group', { name: 'Provider' })).getByRole('button', {
        name: 'Tripo',
      })
    );
    await userEvent.click(
      within(panel).getByRole('button', { name: 'Custom settings for Step ring' })
    );
    const own = within(panel).getByRole('group', { name: 'Step ring settings' });
    const faces = within(own).getByRole('textbox', { name: 'Face limit' });
    await userEvent.clear(faces);
    await userEvent.type(faces, '12000{Enter}');
    await userEvent.click(within(own).getByRole('switch', { name: 'PBR' }));
    expect(within(panel).getByText('Custom')).toBeInTheDocument();
    expect(within(panel).getByText('12,000 faces, no PBR')).toBeInTheDocument();
    expect(within(panel).getByText(/Step ring: 12,000 faces, no PBR/)).toBeInTheDocument();

    await userEvent.click(within(panel).getByRole('button', { name: 'Send 3 to Tripo' }));
    expect(within(panel).getByRole('alert')).toHaveTextContent('Step ring: 12,000 faces, no PBR');
    await userEvent.click(within(panel).getByRole('button', { name: 'Yes, send 3' }));
    await waitFor(() => expect(useProvidersStore.getState().batch).toBeNull());
    const sent = useProvidersStore.getState().jobs.filter(j => j.batch);
    const own3 = Object.fromEntries(
      sent.map(j => [j.assetName, [j.tripo?.faceLimit, j.tripo?.pbr]])
    );
    expect(own3['Step ring']).toEqual([12_000, false]);
    expect(own3['Lantern post']?.[1]).toBe(true);
    expect(own3['Lantern post']?.[0]).not.toBe(12_000);
  });

  it('clears an asset back to the batch settings', async () => {
    const panel = await openBatch(['lantern-post', 'sigil-door']);
    await userEvent.click(
      within(panel).getByRole('button', { name: 'Custom settings for Lantern post' })
    );
    const own = within(panel).getByRole('group', { name: 'Lantern post settings' });
    await userEvent.click(within(own).getByRole('switch', { name: 'Multi-view' }));
    expect(within(panel).getByText('Custom')).toBeInTheDocument();
    await userEvent.click(within(own).getByRole('button', { name: 'Use batch settings' }));
    expect(within(panel).queryByText('Custom')).toBeNull();
  });
  it('closes at once when nothing changed', async () => {
    await openBatch(['lantern-post', 'sigil-door']);
    await userEvent.keyboard('{Escape}');
    expect(useProvidersStore.getState().batch).toBeNull();
  });

  it('asks before Escape or a backdrop click drops edited settings', async () => {
    const panel = await openBatch(['lantern-post', 'sigil-door']);
    await userEvent.click(
      within(panel).getByRole('button', { name: 'Custom settings for Lantern post' })
    );
    const own = within(panel).getByRole('group', { name: 'Lantern post settings' });
    await userEvent.click(within(own).getByRole('switch', { name: 'Multi-view' }));

    await userEvent.keyboard('{Escape}');
    const ask = await screen.findByRole('dialog', { name: 'Discard changes?' });
    expect(useProvidersStore.getState().batch).not.toBeNull();
    await userEvent.click(within(ask).getByRole('button', { name: 'Keep editing' }));
    expect(screen.queryByRole('dialog', { name: 'Discard changes?' })).toBeNull();
    expect(useProvidersStore.getState().batch).not.toBeNull();

    await userEvent.click(panel.parentElement as HTMLElement);
    await userEvent.click(
      within(await screen.findByRole('dialog', { name: 'Discard changes?' })).getByRole('button', {
        name: 'Discard',
      })
    );
    expect(useProvidersStore.getState().batch).toBeNull();
  });

  it('closes at once from Cancel even with edits', async () => {
    const panel = await openBatch(['lantern-post', 'sigil-door']);
    await userEvent.click(
      within(panel).getByRole('button', { name: 'Custom settings for Lantern post' })
    );
    await userEvent.click(
      within(within(panel).getByRole('group', { name: 'Lantern post settings' })).getByRole(
        'switch',
        {
          name: 'Multi-view',
        }
      )
    );
    await userEvent.click(within(panel).getByRole('button', { name: 'Cancel' }));
    expect(useProvidersStore.getState().batch).toBeNull();
    expect(screen.queryByRole('dialog', { name: 'Discard changes?' })).toBeNull();
  });

  it('does not ask after a successful send', async () => {
    const panel = await openBatch(['lantern-post', 'sigil-door']);
    await userEvent.click(
      within(panel).getByRole('button', { name: 'Custom settings for Lantern post' })
    );
    await userEvent.click(
      within(within(panel).getByRole('group', { name: 'Lantern post settings' })).getByRole(
        'switch',
        {
          name: 'Multi-view',
        }
      )
    );
    await userEvent.click(within(panel).getByRole('button', { name: 'Write 2 handoff folders' }));
    await waitFor(() => expect(useProvidersStore.getState().batch).toBeNull());
    expect(screen.queryByRole('dialog', { name: 'Discard changes?' })).toBeNull();
  });

  it('redoes several inbox models, each from its own send, and says what it costs', async () => {
    await openMockWorkspace('configured', undefined, undefined, 'redo');
    await useProvidersStore.getState().load('frostveil');
    render(<BatchSendPanel />);
    await useProvidersStore.getState().openBatchRedo('frostveil', [
      { asset: 'elsheria', job: 'elsheria-3' },
      { asset: 'veil-gate', job: 'veil-gate-2' },
    ]);
    const panel = await screen.findByRole('dialog', { name: 'Redo 2 models' });
    expect(within(panel).getByText('Redo of 2 models from the Inbox')).toBeInTheDocument();
    expect(within(panel).getByText('Redo of elsheria-3')).toBeInTheDocument();
    expect(within(panel).getByText('Redo of veil-gate-2')).toBeInTheDocument();
    // A rigged character with detailed textures (85) and a textured prop (30).
    expect(await within(panel).findByText('About 115 credits')).toBeInTheDocument();
    await userEvent.click(within(panel).getByRole('button', { name: 'Send 2 to Tripo' }));
    expect(within(panel).getByRole('alert')).toHaveTextContent('About 115 credits in all.');
  });
});
