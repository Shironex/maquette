import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks } from '@/test/mock-ipc';
import { openProviders } from '../../openProviders';
import { BatchForm } from '.';

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ project: null, jobs: [], batch: null });
});

async function openForm(assets: string[]) {
  await openProviders('batch');
  await useProvidersStore.getState().openBatch('frostveil', assets);
  const batch = useProvidersStore.getState().batch!;
  return render(<BatchForm projectDir={batch.options!.projectDir} draft={batch.draft!} />);
}

describe('the batch form', () => {
  it('opens the custom settings of one asset', async () => {
    await openForm(['lantern-post', 'sigil-door']);
    const custom = screen.getByRole('button', { name: 'Custom settings for Lantern post' });
    await userEvent.click(custom);
    expect(custom).toHaveAttribute('aria-expanded', 'true');
    const group = screen.getByRole('group', { name: 'Lantern post settings' });
    expect(within(group).getByRole('button', { name: 'Use batch settings' })).toBeDisabled();
  });

  it('offers a tab per kind once the batch mixes them', async () => {
    await openForm(['lantern-post', 'sigil-door']);
    expect(screen.queryByRole('group', { name: 'Preset to edit' })).toBeNull();
    await userEvent.click(
      within(screen.getByRole('group', { name: 'Sigil door kind' })).getByRole('button', {
        name: 'Character',
      })
    );
    const batch = useProvidersStore.getState().batch!;
    render(<BatchForm projectDir={batch.options!.projectDir} draft={batch.draft!} />);
    const tabs = screen.getByRole('group', { name: 'Preset to edit' });
    expect(within(tabs).getByRole('button', { name: 'Props (1)' })).toBeInTheDocument();
    expect(within(tabs).getByRole('button', { name: 'Characters (1)' })).toBeInTheDocument();
  });
});
