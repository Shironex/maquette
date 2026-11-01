import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import type { JobState, Provider, ProviderJob } from '@/ipc/client';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks } from '@/test/mock-ipc';
import { openProviders } from '../openProviders';
import { SendsPanel } from '.';

const job = (id: string, provider: Provider, state: JobState) =>
  ({
    ...useProvidersStore.getState().jobs[0],
    id,
    assetName: id,
    provider,
    state,
    card: null,
    batch: null,
  }) as ProviderJob;

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ project: null, jobs: [], queueOpen: false, checklistOpen: false });
});

function withJobs(...jobs: ProviderJob[]) {
  useProvidersStore.setState({ jobs });
  render(<SendsPanel />);
}

describe('the Sends panel', () => {
  it('stays closed until a list is asked for', () => {
    render(<SendsPanel />);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('has no tabs with only paid sends under way', async () => {
    await openProviders();
    const paid = job('a', 'tripo', 'running');
    useProvidersStore.setState({ queueOpen: true });
    withJobs(paid);
    const sheet = screen.getByRole('dialog', { name: 'Sends' });
    expect(within(sheet).queryByRole('tablist')).toBeNull();
    expect(within(sheet).getByRole('region', { name: 'Sends' })).toBeInTheDocument();
    expect(within(sheet).getByText('1 send in flight')).toBeInTheDocument();
  });

  it('has no tabs with only By hand sends under way', async () => {
    await openProviders();
    useProvidersStore.setState({ checklistOpen: true });
    withJobs(job('a', 'by-hand', 'waiting'));
    const sheet = screen.getByRole('dialog', { name: 'By hand checklist' });
    expect(within(sheet).queryByRole('tablist')).toBeNull();
    expect(within(sheet).getByText('1 model to make · no API credits')).toBeInTheDocument();
  });

  it('switches between the lists with both kinds under way', async () => {
    await openProviders();
    useProvidersStore.setState({ queueOpen: true });
    withJobs(job('a', 'by-hand', 'waiting'), job('b', 'meshy', 'queued'));
    const queueTab = screen.getByRole('tab', { name: /^Sends\s*2$/ });
    expect(queueTab).toHaveAttribute('aria-selected', 'true');
    await userEvent.click(screen.getByRole('tab', { name: /^By hand\s*1$/ }));
    expect(screen.getByRole('dialog', { name: 'By hand checklist' })).toBeInTheDocument();
    expect(useProvidersStore.getState().queueOpen).toBe(false);
    await userEvent.keyboard('{ArrowLeft}');
    expect(screen.getByRole('dialog', { name: 'Sends' })).toBeInTheDocument();
  });

  it('keeps a way to the other list when the open one is no longer the only kind', async () => {
    await openProviders();
    useProvidersStore.setState({ queueOpen: true });
    withJobs(job('a', 'by-hand', 'waiting'));
    expect(screen.getByRole('tablist')).toBeInTheDocument();
  });

  it('shows no By hand tab when nothing is under way and none was sent', async () => {
    await openProviders();
    useProvidersStore.setState({ queueOpen: true });
    withJobs(job('a', 'tripo', 'failed'));
    expect(screen.queryByRole('tablist')).toBeNull();
    expect(screen.getByText('Nothing in flight')).toBeInTheDocument();
  });

  it('closes on Escape and on Done, with either list open', async () => {
    await openProviders();
    useProvidersStore.setState({ queueOpen: true });
    withJobs(job('a', 'tripo', 'running'));
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('dialog')).toBeNull();
    useProvidersStore.getState().setChecklistOpen(true);
    await userEvent.click(await screen.findByRole('button', { name: 'Done' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(useProvidersStore.getState().checklistOpen).toBe(false);
  });
});
