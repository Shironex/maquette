import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks } from '@/test/mock-ipc';
import { openProviders } from '../../openProviders';
import { ChecklistRow } from '.';

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ project: null, jobs: [] });
});

describe('a checklist row', () => {
  it('opens its settings card in place and opens its folder', async () => {
    const { job, sent } = await openProviders();
    render(
      <ul>
        <ChecklistRow job={job('guardian-statues-1')} />
      </ul>
    );
    const toggle = screen.getByRole('button', { name: 'Settings card' });
    await userEvent.click(toggle);
    expect(toggle).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByText('Use these settings')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Hide the settings card' }));
    expect(screen.queryByText('Use these settings')).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Open the Guardian statues folder' }));
    expect(sent('provider_open_handoff')).toEqual([{ id: 'guardian-statues-1' }]);
  });
});
