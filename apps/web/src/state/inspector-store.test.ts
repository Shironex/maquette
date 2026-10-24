import { afterEach, describe, expect, it } from 'vitest';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useInspectorStore } from './inspector-store';

afterEach(() => {
  clearMocks();
  useInspectorStore.setState({ view: null, project: null, selected: null, exporting: null });
});

describe('the Inspector store', () => {
  it('stays on the project opened while an export ran', async () => {
    await openMockWorkspace();
    await useInspectorStore.getState().load('moon-island');
    const model = useInspectorStore.getState().view!.models[0]!.id;
    const exporting = useInspectorStore.getState().exportGlb(model, 'Island');
    const switched = useInspectorStore.getState().load('rooftop-cinema');
    await Promise.all([exporting, switched]);
    expect(useInspectorStore.getState().project).toBe('rooftop-cinema');
    expect(useInspectorStore.getState().view?.project).toBe('rooftop-cinema');
  });

  it('starts the next project with nothing picked and no camera stop', async () => {
    await openMockWorkspace();
    await useInspectorStore.getState().load('moon-island');
    const store = useInspectorStore.getState();
    store.pick({ comparison: 'scene', left: 'v6', right: 'v7' });
    store.flyTo(3);
    await useInspectorStore.getState().load('rooftop-cinema');
    expect(useInspectorStore.getState()).toMatchObject({
      comparison: null,
      left: null,
      right: null,
      stop: null,
    });
  });
});
