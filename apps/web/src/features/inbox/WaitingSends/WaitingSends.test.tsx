import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { WaitingSends } from '.';

beforeEach(async () => {
  await openMockWorkspace('configured');
  await useProvidersStore.getState().load('frostveil');
});

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ project: null, jobs: [] });
});

describe('the By hand sends still waiting', () => {
  it('lists each waiting send and stops waiting for one', async () => {
    render(<WaitingSends />);
    expect(screen.getByRole('heading', { name: /Still waiting · 1 send/ })).toBeInTheDocument();
    expect(screen.getByText('Guardian statues still waits')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Open handoff' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Stop waiting' }));
    await vi.waitFor(() =>
      expect(
        useProvidersStore.getState().jobs.find(j => j.id === 'guardian-statues-1')?.state
      ).not.toBe('waiting')
    );
  });

  it('shows nothing when no By hand send waits', () => {
    useProvidersStore.setState({ jobs: [] });
    const { container } = render(<WaitingSends />);
    expect(container).toBeEmptyDOMElement();
  });
});
