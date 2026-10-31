import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks } from '@/test/mock-ipc';
import { openProviders } from '../openProviders';
import { ProviderQueue } from '.';

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ project: null, jobs: [] });
});

describe('the 3D queue', () => {
  it('lists the sends and clears the finished ones through the backend', async () => {
    const { sent } = await openProviders();
    render(<ProviderQueue onClose={() => {}} />);
    const queue = screen.getByRole('region', { name: '3D queue' });
    expect(within(queue).getByText('Veil gate')).toBeInTheDocument();
    await userEvent.click(within(queue).getByRole('button', { name: 'Clear finished' }));
    expect(sent('provider_clear_finished')).toHaveLength(1);
  });

  it('says so when nothing was sent', () => {
    render(<ProviderQueue onClose={() => {}} />);
    expect(
      screen.getByText(
        'Nothing sent yet. Send an asset from its card or from a picked concept in the Lab.'
      )
    ).toBeInTheDocument();
  });
});
