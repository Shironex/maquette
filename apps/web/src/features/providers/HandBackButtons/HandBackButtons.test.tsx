import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks } from '@/test/mock-ipc';
import { openProviders } from '../openProviders';
import { HandBackButtons } from '.';

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ project: null, jobs: [], downloads: null, busy: [] });
});

describe('the hand back buttons', () => {
  it('check Downloads for the waiting sends', async () => {
    const { job, sent } = await openProviders();
    render(<HandBackButtons job={job('lanterns-1')} />);
    await userEvent.click(screen.getByRole('button', { name: 'Check Downloads' }));
    expect(sent('provider_check_downloads')).toHaveLength(1);
  });

  it('leave Check Downloads out when asked', async () => {
    const { job } = await openProviders();
    render(<HandBackButtons job={job('lanterns-1')} downloads={false} />);
    expect(screen.getByRole('button', { name: 'Add model...' })).toBeEnabled();
    expect(screen.queryByRole('button', { name: 'Check Downloads' })).toBeNull();
  });
});
