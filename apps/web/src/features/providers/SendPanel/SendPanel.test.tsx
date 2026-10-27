import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { SendPanel } from '.';

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ project: null, jobs: [], panel: null, sending: false });
});

async function openPanel() {
  await openMockWorkspace('configured', undefined, 'batch');
  await useProvidersStore.getState().load('frostveil');
  render(<SendPanel />);
  await useProvidersStore.getState().openPanel('frostveil', 'lantern-post');
  return screen.findByRole('dialog', { name: 'Send Lantern post to 3D' });
}

const ASK = { name: 'Discard changes?' };

describe('the Send to 3D panel', () => {
  it('closes at once on Escape when nothing changed', async () => {
    await openPanel();
    await userEvent.keyboard('{Escape}');
    expect(useProvidersStore.getState().panel).toBeNull();
  });

  it('asks before Escape drops edited settings, and Keep editing stays', async () => {
    const panel = await openPanel();
    await userEvent.click(within(panel).getByRole('switch', { name: 'PBR' }));
    await userEvent.keyboard('{Escape}');
    const ask = await screen.findByRole('dialog', ASK);
    await userEvent.click(within(ask).getByRole('button', { name: 'Keep editing' }));
    expect(screen.queryByRole('dialog', ASK)).toBeNull();
    expect(useProvidersStore.getState().panel).not.toBeNull();

    await userEvent.keyboard('{Escape}');
    await userEvent.click(
      within(await screen.findByRole('dialog', ASK)).getByRole('button', { name: 'Discard' })
    );
    expect(useProvidersStore.getState().panel).toBeNull();
  });

  it('asks on a backdrop click too, but not once the edit is undone', async () => {
    const panel = await openPanel();
    const pbr = within(panel).getByRole('switch', { name: 'PBR' });
    await userEvent.click(pbr);
    await userEvent.click(panel.parentElement as HTMLElement);
    await userEvent.click(
      within(await screen.findByRole('dialog', ASK)).getByRole('button', { name: 'Keep editing' })
    );
    await userEvent.click(pbr);
    await userEvent.click(panel.parentElement as HTMLElement);
    expect(screen.queryByRole('dialog', ASK)).toBeNull();
    expect(useProvidersStore.getState().panel).toBeNull();
  });

  it('closes at once from Cancel with edits', async () => {
    const panel = await openPanel();
    await userEvent.click(within(panel).getByRole('switch', { name: 'PBR' }));
    await userEvent.click(within(panel).getByRole('button', { name: 'Cancel' }));
    expect(useProvidersStore.getState().panel).toBeNull();
    expect(screen.queryByRole('dialog', ASK)).toBeNull();
  });

  it('closes without asking after a send', async () => {
    const panel = await openPanel();
    await userEvent.click(within(panel).getByRole('switch', { name: 'PBR' }));
    await userEvent.click(
      within(within(panel).getByRole('group', { name: 'Provider' })).getByRole('button', {
        name: 'Tripo',
      })
    );
    await userEvent.click(within(panel).getByRole('button', { name: 'Send to Tripo' }));
    await waitFor(() => expect(useProvidersStore.getState().panel).toBeNull());
    expect(screen.queryByRole('dialog', ASK)).toBeNull();
  });
});
