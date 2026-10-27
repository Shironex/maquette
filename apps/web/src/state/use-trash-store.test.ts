import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useAppStore } from './use-app-store';
import { useProvidersStore } from './use-providers-store';
import { useTrashStore } from './use-trash-store';

beforeEach(async () => {
  await openMockWorkspace();
  await useProvidersStore.getState().load('frostveil');
});

afterEach(() => {
  clearMocks();
  useTrashStore.setState({ project: null, entries: [], restoring: null });
  useProvidersStore.setState({ project: null, jobs: [] });
  useAppStore.setState({ notice: null });
});

const board = () => (useAppStore.getState().workspace?.project?.assets ?? []).map(a => a.id);

describe('useTrashStore', () => {
  it('removes assets, stops their waiting sends and undoes it', async () => {
    expect(board()).toContain('guardian-statues');
    const done = await useTrashStore.getState().remove('frostveil', ['guardian-statues', 'nope']);
    expect(done.outcomes.map(o => o.state)).toEqual(['removed', 'not-found']);
    expect(done.outcomes[0]?.stopped).toEqual(['guardian-statues-1']);
    expect(board()).not.toContain('guardian-statues');
    expect(useTrashStore.getState().entries.map(e => e.name)).toEqual(['Guardian statues']);
    const notice = useAppStore.getState().notice;
    expect(notice?.title).toBe('Removed Guardian statues');
    expect(notice?.detail).toContain('1 was already gone.');
    await waitForStoppedSend();

    notice?.action?.run();
    await vi.waitFor(() => expect(board()).toContain('guardian-statues'));
    await vi.waitFor(() =>
      expect(useAppStore.getState().notice?.title).toBe('Restored Guardian statues')
    );
    expect(useTrashStore.getState().entries).toEqual([]);
  });

  it('keeps an asset whose paid send is being made', async () => {
    const done = await useTrashStore.getState().remove('frostveil', ['veil-gate']);
    expect(done.outcomes[0]?.state).toBe('refused');
    expect(board()).toContain('veil-gate');
    expect(useAppStore.getState().notice).toMatchObject({
      status: 'failed',
      title: 'Could not remove Veil gate',
    });
    expect(useAppStore.getState().notice?.action).toBeUndefined();
  });
});

/** The stopped send shows as cancelled once the queue reloads. */
async function waitForStoppedSend() {
  await vi.waitFor(() =>
    expect(useProvidersStore.getState().jobs.find(j => j.id === 'guardian-statues-1')?.state).toBe(
      'cancelled'
    )
  );
}
