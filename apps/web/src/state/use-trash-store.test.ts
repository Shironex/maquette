import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useAppStore } from './use-app-store';
import { useProvidersStore } from './use-providers-store';
import { useTrashStore } from './use-trash-store';

beforeEach(async () => {
  await openMockWorkspace();
  await useProvidersStore.getState().load('moon-island');
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
    expect(board()).toContain('lanterns');
    const done = await useTrashStore.getState().remove('moon-island', ['lanterns', 'nope']);
    expect(done.outcomes.map(o => o.state)).toEqual(['removed', 'not-found']);
    expect(done.outcomes[0]?.stopped).toEqual(['lanterns-1']);
    expect(board()).not.toContain('lanterns');
    expect(useTrashStore.getState().entries.map(e => e.name)).toEqual(['Stone lanterns']);
    const notice = useAppStore.getState().notice;
    expect(notice?.title).toBe('Removed Stone lanterns');
    expect(notice?.detail).toContain('1 was already gone.');
    await waitForStoppedSend();

    notice?.action?.run();
    await vi.waitFor(() => expect(board()).toContain('lanterns'));
    await vi.waitFor(() =>
      expect(useAppStore.getState().notice?.title).toBe('Restored Stone lanterns')
    );
    expect(useTrashStore.getState().entries).toEqual([]);
  });

  it('keeps an asset whose paid send is being made', async () => {
    const done = await useTrashStore.getState().remove('moon-island', ['sakura']);
    expect(done.outcomes[0]?.state).toBe('refused');
    expect(board()).toContain('sakura');
    expect(useAppStore.getState().notice).toMatchObject({
      status: 'failed',
      title: 'Could not remove Sakura',
    });
    expect(useAppStore.getState().notice?.action).toBeUndefined();
  });
});

/** The stopped send shows as cancelled once the queue reloads. */
async function waitForStoppedSend() {
  await vi.waitFor(() =>
    expect(useProvidersStore.getState().jobs.find(j => j.id === 'lanterns-1')?.state).toBe(
      'cancelled'
    )
  );
}
