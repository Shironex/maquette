import { afterEach, describe, expect, it } from 'vitest';
import type { Asset, ProjectManifest, ThumbnailReady } from '@/ipc/client';
import { useAppStore } from './use-app-store';
import { useGameStore } from './use-game-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';

const lantern = {
  id: 'lantern',
  versions: [{ version: 1, file: 'models/lantern_v1.glb' }],
} as unknown as Asset;

const done = (state: ThumbnailReady['state'], file = '/ws/projects/p/models/lantern_v1.glb') =>
  ({ file, state, thumbnail: null, project: null }) as unknown as ThumbnailReady;

afterEach(() => {
  clearMocks();
  useGameStore.setState({ rendering: false, waiting: 0, waitingFor: null, failedPreviews: 0 });
  useAppStore.setState({ notice: null });
});

describe('render previews progress', () => {
  async function open(waiting: number) {
    await openMockWorkspace();
    const project = useAppStore.getState().workspace!.project!;
    useAppStore.getState().setProject({ ...project, assets: [lantern] } as ProjectManifest);
    useGameStore.setState({ waiting, waitingFor: project.id });
  }

  it('reports the failures once the last queued render ends', async () => {
    await open(2);
    useGameStore.getState().onThumbnail(done('fallback'));
    expect(useGameStore.getState().waiting).toBe(1);
    expect(useAppStore.getState().notice).toBeNull();
    useGameStore.getState().onThumbnail(done('ready'));
    expect(useGameStore.getState().waiting).toBe(0);
    expect(useAppStore.getState().notice).toMatchObject({
      status: 'failed',
      title: '1 preview could not render',
      detail: 'Render previews to retry.',
    });
  });

  it('stays quiet for renders of other files and for the apply events', async () => {
    await open(1);
    useGameStore.getState().onThumbnail(done('fallback', '/ws/inbox/other.glb'));
    useGameStore.getState().onThumbnail({ ...done('ready'), project: {} as ProjectManifest });
    expect(useGameStore.getState().waiting).toBe(1);
    expect(useAppStore.getState().notice).toBeNull();
  });
});
