import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands, type RefWarning } from '@/ipc/client';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockRefs } from '@/test/mock-ipc';
import { openProviders } from '../../openProviders';
import { BatchForm } from '.';

const modelNote: RefWarning = {
  kind: 'has-model',
  file: null,
  view: null,
  title: 'Guardian statues already has a model (v1, from the library entry guardian-statues)',
  detail: 'This send makes v2; the library entry is not changed. The older versions stay.',
  fixes: [],
};

afterEach(() => {
  vi.restoreAllMocks();
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

  it('counts the assets with ref warnings, filters to them and repeats it at approval', async () => {
    await openMockRefs();
    await useProvidersStore.getState().loadKeys();
    await useProvidersStore.getState().openBatch('frostveil', ['guardian-statues', 'veil-gate']);
    const draw = () => {
      const batch = useProvidersStore.getState().batch!;
      return render(<BatchForm projectDir={batch.options!.projectDir} draft={batch.draft!} />);
    };
    const { unmount } = draw();
    expect(await screen.findByText(/1 of 2 assets has ref warnings/)).toBeInTheDocument();
    expect(screen.getByText('Refs look good')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /1 ref warning: guardian_statues/ }));
    expect(screen.getByRole('button', { name: 'Leave out this image' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Show only these' }));
    expect(screen.queryByText('Veil gate')).toBeNull();
    unmount();
    useProvidersStore.getState().setBatch({ provider: 'tripo' });
    await useProvidersStore.getState().sendBatch();
    draw();
    expect(await screen.findByText(/1 asset goes with ref warnings\./)).toBeInTheDocument();
  });

  it('notes the asset that has a model already without counting it as a ref warning', async () => {
    await openMockRefs();
    vi.spyOn(commands, 'refsWarnings').mockResolvedValue([[modelNote], []]);
    await useProvidersStore.getState().loadKeys();
    await useProvidersStore.getState().openBatch('frostveil', ['guardian-statues', 'veil-gate']);
    const batch = useProvidersStore.getState().batch!;
    render(<BatchForm projectDir={batch.options!.projectDir} draft={batch.draft!} />);
    const notes = await screen.findAllByRole('note', { name: 'Existing model' });
    expect(notes).toHaveLength(1);
    expect(notes[0]).toHaveTextContent('This send makes v2');
    expect(screen.queryByText(/assets? (has|have) ref warnings/)).toBeNull();
  });
});
