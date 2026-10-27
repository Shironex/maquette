import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks } from '@/test/mock-ipc';
import { openProviders } from '../../openProviders';
import { QueueRow } from '.';

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ project: null, jobs: [], panel: null });
});

function row(job: Parameters<typeof QueueRow>[0]['job']) {
  render(
    <ul>
      <QueueRow job={job} now={job.createdAtMs ?? 0} />
    </ul>
  );
}

describe('a queue row', () => {
  it('warns that a running Tripo send cannot be cancelled', async () => {
    const { job } = await openProviders();
    row(job('veil-gate-1'));
    expect(
      screen.getByText(
        'Tripo cannot cancel a sent task, so this one finishes and lands in the Inbox.'
      )
    ).toBeInTheDocument();
    expect(screen.getByRole('progressbar')).toBeInTheDocument();
  });

  it('opens the settings card of a by hand send and stops waiting through the backend', async () => {
    const { job, sent } = await openProviders();
    row(job('guardian-statues-1'));
    await userEvent.click(screen.getByRole('button', { name: 'Settings card' }));
    expect(useProvidersStore.getState().panel?.handoff?.id).toBe('guardian-statues-1');
    await userEvent.click(screen.getByRole('button', { name: 'Stop waiting' }));
    expect(sent('provider_cancel')).toEqual([{ id: 'guardian-statues-1' }]);
  });
});
