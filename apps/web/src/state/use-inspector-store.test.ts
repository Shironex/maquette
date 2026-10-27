import { afterEach, describe, expect, it } from 'vitest';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useInspectorStore } from './use-inspector-store';

afterEach(() => {
  clearMocks();
  useInspectorStore.setState({ view: null, project: null, selected: null, exporting: null });
});

describe('the Inspector store', () => {
  it('stays on the project opened while an export ran', async () => {
    await openMockWorkspace();
    await useInspectorStore.getState().load('frostveil');
    const model = useInspectorStore.getState().view!.models[0]!.id;
    const exporting = useInspectorStore.getState().exportGlb(model, 'Veil platform');
    const switched = useInspectorStore.getState().load('frost-cathedral');
    await Promise.all([exporting, switched]);
    expect(useInspectorStore.getState().project).toBe('frost-cathedral');
    expect(useInspectorStore.getState().view?.project).toBe('frost-cathedral');
  });

  it('starts the next project with nothing picked and no camera stop', async () => {
    await openMockWorkspace();
    await useInspectorStore.getState().load('frostveil');
    const store = useInspectorStore.getState();
    store.pick({ comparison: 'scene', left: 'v6', right: 'v7' });
    store.flyTo(3);
    await useInspectorStore.getState().load('frost-cathedral');
    expect(useInspectorStore.getState()).toMatchObject({
      comparison: null,
      left: null,
      right: null,
      stop: null,
    });
  });
});
