import { render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { jobTitle } from '@/state/providers';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks } from '@/test/mock-ipc';
import { openProviders } from '../../../openProviders';
import { EarlierSends } from '.';

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ project: null, jobs: [] });
});

describe('the earlier sends', () => {
  it('show each send with its message and status', async () => {
    const { job } = await openProviders();
    render(<EarlierSends jobs={[job('lanterns-1')]} />);
    const lanterns = job('lanterns-1');
    const row = screen.getByRole('listitem');
    expect(within(row).getByText(jobTitle(lanterns))).toBeInTheDocument();
    // The message line and the status chip.
    expect(within(row).getAllByText('Waiting for your model')).toHaveLength(2);
  });
});
