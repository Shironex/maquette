import { afterEach, describe, expect, it, vi } from 'vitest';
import { registerMockFile } from '@/ipc/files';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useInboxStore } from './inbox-store';

const snapshotModel = vi.fn(async () => new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10]));
vi.mock('@/features/viewer/snapshot', () => ({ snapshotModel }));

afterEach(() => {
  clearMocks();
  useInboxStore.setState({ snapshot: null, selected: null });
});

describe('inbox snapshots', () => {
  it('snapshot a new file under a name already snapshotted', async () => {
    await openMockWorkspace();
    await useInboxStore.getState().load();
    const item = useInboxStore
      .getState()
      .snapshot!.items.find(i => i.thumbnailState === 'fallback')!;
    registerMockFile(item.file, '/mock/model.glb');
    // The download finished: a readable file Blender will not render.
    const replaced = {
      ...item,
      state: 'ready' as const,
      bytes: (item.bytes ?? 0) + 1,
      modifiedMs: (item.modifiedMs ?? 0) + 1,
    };
    const swap = (next: typeof item) => {
      const snapshot = useInboxStore.getState().snapshot!;
      useInboxStore.setState({
        snapshot: { ...snapshot, items: snapshot.items.map(i => (i.id === item.id ? next : i)) },
      });
    };
    swap(replaced);
    await useInboxStore.getState().snapshotMissing();
    expect(snapshotModel).toHaveBeenCalledTimes(1);

    // The download is replaced again under the same name.
    swap({ ...replaced, bytes: replaced.bytes + 1, thumbnail: null, thumbnailState: 'fallback' });
    await useInboxStore.getState().snapshotMissing();
    expect(snapshotModel).toHaveBeenCalledTimes(2);

    // The same content is not tried twice.
    await useInboxStore.getState().snapshotMissing();
    expect(snapshotModel).toHaveBeenCalledTimes(2);
  });

  it('ignores a thumbnail event for older content under the same name', async () => {
    await openMockWorkspace();
    await useInboxStore.getState().load();
    const item = useInboxStore.getState().snapshot!.items[0]!;
    const event = {
      file: item.file,
      thumbnail: '/thumbs/old.png',
      state: 'ready' as const,
      project: null,
    };
    const thumbnail = () => useInboxStore.getState().snapshot!.items[0]!.thumbnail;
    const before = thumbnail();

    // A render of the file as it was before it was replaced.
    useInboxStore
      .getState()
      .onThumbnail({ ...event, bytes: (item.bytes ?? 0) - 1, modifiedMs: item.modifiedMs });
    expect(thumbnail()).toBe(before);

    useInboxStore
      .getState()
      .onThumbnail({ ...event, bytes: item.bytes, modifiedMs: item.modifiedMs });
    expect(thumbnail()).toBe('/thumbs/old.png');
  });
});
