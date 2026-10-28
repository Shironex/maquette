import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useRefsStore } from '@/state/use-refs-store';
import { clearMocks, openMockRefs } from '@/test/mock-ipc';
import { openProviders } from '../../openProviders';
import { SendForm } from '.';

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ project: null, jobs: [], panel: null, keys: [] });
  useRefsStore.setState({ sheet: null, view: null });
});

async function openForm(asset: string) {
  await openProviders();
  await useProvidersStore.getState().openPanel('frostveil', asset);
  const { options, draft } = useProvidersStore.getState().panel!;
  return render(<SendForm options={options!} draft={draft!} />);
}

describe('the Send to 3D form', () => {
  it('lists the asset refs and the earlier sends', async () => {
    await openForm('veil-gate');
    expect(screen.getByRole('heading', { name: 'Earlier sends' })).toBeInTheDocument();
    expect(screen.getByText('veil_gate_front.png')).toBeInTheDocument();
  });

  it('sends a key-less provider to Settings', async () => {
    await openForm('veil-gate');
    useProvidersStore.setState({ keys: [] });
    await userEvent.click(
      within(screen.getByRole('group', { name: 'Provider' })).getByRole('button', {
        name: 'Meshy',
      })
    );
    const { options, draft } = useProvidersStore.getState().panel!;
    render(<SendForm options={options!} draft={draft!} />);
    await userEvent.click(screen.getAllByRole('button', { name: 'Open Settings' })[0]!);
    expect(useAppStore.getState().view).toBe('settings');
    expect(useProvidersStore.getState().panel).toBeNull();
  });

  it('opens a redo prefilled, with the Meshy prompt and what it should cost', async () => {
    await openProviders(undefined, 'redo');
    await useProvidersStore.getState().openRedo('frostveil', 'veil-gate', 'veil-gate-2');
    const { options, draft } = useProvidersStore.getState().panel!;
    render(<SendForm options={options!} draft={draft!} />);
    expect(screen.getByText('Redo of veil-gate-2, sent to Meshy')).toBeInTheDocument();
    expect(screen.getByLabelText('Texture prompt')).toHaveValue(
      'Pale frosted stone, violet rune glow, soft painted look'
    );
    expect(await screen.findByText('About 30 credits')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Redo with Meshy' })).toBeEnabled();
  });

  it('warns about weak refs before the send, counts them and leaves an image out', async () => {
    await openMockRefs();
    await useProvidersStore.getState().openPanel('frostveil', 'guardian-statues');
    const draw = () => {
      const { options, draft } = useProvidersStore.getState().panel!;
      return render(<SendForm options={options!} draft={draft!} />);
    };
    const { unmount } = draw();
    expect(
      await screen.findByText('guardian_statues_front.png crops the object')
    ).toBeInTheDocument();
    expect(screen.getByText('1 ref warning')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Send to|Write the handoff folder/ })).toBeEnabled();
    await userEvent.click(screen.getByRole('button', { name: 'Leave out this image' }));
    expect(useProvidersStore.getState().panel!.draft!.images).toEqual([]);
    unmount();
  });

  it('opens Clean background from a warning', async () => {
    await openMockRefs();
    await useProvidersStore.getState().openPanel('frostveil', 'guardian-statues');
    const { options, draft } = useProvidersStore.getState().panel!;
    render(<SendForm options={options!} draft={draft!} />);
    await userEvent.click(await screen.findByRole('button', { name: 'Clean background' }));
    expect(useProvidersStore.getState().panel).toBeNull();
    expect(useRefsStore.getState().sheet).toMatchObject({
      mode: 'clean',
      file: 'refs/guardian_statues_front.png',
    });
  });
});
