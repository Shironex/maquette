import { renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  commands,
  type Asset,
  type PreviewsAsked,
  type ProjectManifest,
  type ThumbnailReady,
} from '@/ipc/client';
import { useAppStore } from './use-app-store';
import { useGameStore, useRendersPreview } from './use-game-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';

const lantern = {
  id: 'lantern',
  versions: [{ version: 1, file: 'models/lantern_v1.glb' }],
} as unknown as Asset;

const foxVersion = { version: 1, file: 'models/fox_v1.glb' };

const done = (state: ThumbnailReady['state'], file = '/ws/projects/p/models/lantern_v1.glb') =>
  ({ file, state, thumbnail: null, project: null }) as unknown as ThumbnailReady;

const fileOf = (project: string, name = 'lantern') =>
  `/ws/projects/${project}/models/${name}_v1.glb`;

const asked = (renderingFiles: string[], over: object = {}) =>
  ({
    asked: renderingFiles.length,
    rendering: renderingFiles.length,
    renderingFiles,
    failed: 0,
    reason: null,
    project: { ...useAppStore.getState().workspace!.project!, assets: [lantern] },
    ...over,
  }) as PreviewsAsked;

afterEach(() => {
  vi.restoreAllMocks();
  clearMocks();
  useGameStore.setState({ rendering: false, queued: [], waitingFor: null, failedPreviews: 0 });
  useAppStore.setState({ notice: null });
});

describe('render previews progress', () => {
  async function open(queued: string[]) {
    await openMockWorkspace();
    const project = useAppStore.getState().workspace!.project!;
    useAppStore.getState().setProject({ ...project, assets: [lantern] } as ProjectManifest);
    useGameStore.setState({ queued, waitingFor: project.id });
    return project.id;
  }

  it('reports the failures once the last queued render ends', async () => {
    await open([fileOf('p'), fileOf('p', 'fox')]);
    useGameStore.getState().onThumbnail(done('fallback'));
    expect(useGameStore.getState().queued).toEqual([fileOf('p', 'fox')]);
    expect(useAppStore.getState().notice).toBeNull();
    useGameStore.getState().onThumbnail(done('ready', fileOf('p', 'fox')));
    expect(useGameStore.getState().queued).toEqual([]);
    expect(useGameStore.getState().waitingFor).toBeNull();
    expect(useAppStore.getState().notice).toMatchObject({
      status: 'failed',
      title: '1 preview could not render',
      detail: 'Render previews to retry.',
    });
  });

  it('stays quiet for renders of other files and for the apply events', async () => {
    await open([fileOf('p')]);
    useGameStore.getState().onThumbnail(done('fallback', '/ws/inbox/other.glb'));
    useGameStore.getState().onThumbnail({ ...done('ready'), project: {} as ProjectManifest });
    expect(useGameStore.getState().queued).toEqual([fileOf('p')]);
    expect(useAppStore.getState().notice).toBeNull();
  });

  it('ends after the person opened another project while the renders ran', async () => {
    await open([fileOf('p')]);
    const workspace = useAppStore.getState().workspace!;
    const other = { ...workspace.project!, id: 'other', assets: [] };
    useAppStore.setState({ workspace: { ...workspace, project: other } });
    useGameStore.getState().onThumbnail(done('ready', fileOf('p')));
    expect(useGameStore.getState().waitingFor).toBeNull();
    expect(useGameStore.getState().queued).toEqual([]);
  });

  it('counts a render that ended while the service was still answering', async () => {
    const id = await open([]);
    let answer: (a: PreviewsAsked) => void = () => {};
    vi.spyOn(commands, 'thumbnailRenderMissing').mockReturnValue(
      new Promise(resolve => {
        answer = resolve;
      })
    );
    const going = useGameStore.getState().renderPreviews(id);
    useGameStore.getState().onThumbnail(done('ready', fileOf('p')));
    answer(asked([fileOf('p'), fileOf('p', 'fox')]));
    await going;
    expect(useGameStore.getState().queued).toEqual([fileOf('p', 'fox')]);
  });

  describe('on a card', () => {
    const says = (asset: Asset, project: string) =>
      renderHook(() => useRendersPreview(asset, project)).result.current;

    it('says so only for a model still queued, not one the service could not queue', async () => {
      const id = await open([]);
      vi.spyOn(commands, 'thumbnailRenderMissing').mockResolvedValue(
        asked([fileOf('p', 'fox')], { failed: 1, reason: 'Unreadable' })
      );
      await useGameStore.getState().renderPreviews(id);
      expect(says(lantern, id)).toBe(false);
      expect(says({ ...lantern, id: 'fox', versions: [foxVersion] } as Asset, id)).toBe(true);
    });

    it('stops saying so when the render failed and its fallback event came', async () => {
      const id = await open([fileOf('p'), fileOf('p', 'fox')]);
      expect(says(lantern, id)).toBe(true);
      useGameStore.getState().onThumbnail(done('fallback'));
      expect(says(lantern, id)).toBe(false);
    });

    it('says nothing on a card of another project, even while asking', async () => {
      const id = await open([]);
      useGameStore.setState({ rendering: true, waitingFor: id });
      expect(says(lantern, id)).toBe(true);
      expect(says(lantern, 'other')).toBe(false);
    });

    it('does not take a longer name for the same file', async () => {
      const id = await open(['/ws/projects/p/xmodels/lantern_v1.glb']);
      expect(says(lantern, id)).toBe(false);
    });
  });
});
