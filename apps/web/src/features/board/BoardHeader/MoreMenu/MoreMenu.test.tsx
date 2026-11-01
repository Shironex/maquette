import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  commands,
  type Asset,
  type JobState,
  type Provider,
  type ProjectManifest,
} from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { forgetStoredPrefs, useBoardPrefsStore } from '@/state/use-board-prefs-store';
import { useGameStore } from '@/state/use-game-store';
import { usePackStore } from '@/state/use-pack-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useTrashStore } from '@/state/use-trash-store';
import { clearMocks, openMockGame, openMockWorkspace } from '@/test/mock-ipc';
import MoreMenu from './MoreMenu';

const openMore = () => userEvent.click(screen.getByRole('button', { name: 'More board actions' }));
const job = (id: string, provider: Provider, state: JobState) =>
  ({ id, provider, state, assetName: id, batch: null }) as never;

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, 'batch');
  await useProvidersStore.getState().load('frostveil');
});

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({
    project: null,
    jobs: [],
    queueOpen: false,
    checklistOpen: false,
    busy: [],
  });
  useTrashStore.setState({ project: null, entries: [], restoring: null });
  useAppStore.setState({ notice: null });
  window.localStorage.clear();
  forgetStoredPrefs();
  useBoardPrefsStore.setState({ byProject: {} });
});

describe('the more menu', () => {
  it('opens the pack sheet on the project from Export pack', async () => {
    const before = usePackStore.getState().show;
    const show = vi.fn(async () => {});
    usePackStore.setState({ show } as never);
    render(<MoreMenu />);
    await openMore();
    const pack = screen.getByRole('menuitem', { name: 'Export pack' });
    await userEvent.click(pack);
    expect(show).toHaveBeenCalledWith(
      { from: 'project', project: 'frostveil' },
      { project: 'Frostveil' }
    );
    expect(useAppStore.getState().view).toBe('library');
    usePackStore.setState({ show: before });
  });

  it('reaches every send from All sends, with nothing in flight too', async () => {
    useProvidersStore.setState({ jobs: [job('a', 'tripo', 'failed')] });
    render(<MoreMenu />);
    await openMore();
    await userEvent.click(screen.getByRole('menuitem', { name: 'All sends, 1 failed' }));
    expect(useProvidersStore.getState().queueOpen).toBe(true);
  });

  it('has no Removed and no dot while the trash is empty', async () => {
    render(<MoreMenu />);
    await vi.waitFor(() => expect(useTrashStore.getState().project).toBe('frostveil'));
    expect(screen.queryByTestId('removed-dot')).toBeNull();
    await openMore();
    expect(screen.queryByRole('menuitem', { name: /^Removed/ })).toBeNull();
  });

  it('counts Removed, shows the dot and restores from the same list as before', async () => {
    await useTrashStore.getState().remove('frostveil', ['elsheria']);
    render(<MoreMenu />);
    await screen.findByTestId('removed-dot');
    await openMore();
    await userEvent.click(screen.getByRole('menuitem', { name: /^Removed\s*1$/ }));
    const menu = await screen.findByRole('menu', { name: 'Recently removed' });
    await userEvent.click(within(menu).getByRole('menuitem', { name: 'Elsheria' }));
    await vi.waitFor(() =>
      expect(
        (useAppStore.getState().workspace?.project?.assets ?? []).some(a => a.id === 'elsheria')
      ).toBe(true)
    );
    await vi.waitFor(() => expect(screen.queryByTestId('removed-dot')).toBeNull());
    expect(screen.getByRole('button', { name: 'More board actions' })).toHaveFocus();
  });
});

describe('Render previews', () => {
  const model = (id: string, thumbnail: string | null): Asset => ({
    id,
    name: id,
    kind: 'prop',
    stage: 'blender',
    status: 'review',
    versions: [{ version: 1, file: `models/${id}_v1.glb`, createdAt: '', stats: null }],
    thumbnail,
    createdAt: '',
    updatedAt: '',
    origin: null,
  });
  const withAssets = (assets: Asset[]) => {
    const project = useAppStore.getState().workspace!.project!;
    expect(project.game ?? null).toBeNull();
    useAppStore.getState().setProject({ ...project, assets } as ProjectManifest);
  };

  afterEach(() => {
    vi.restoreAllMocks();
    useGameStore.setState({ rendering: false, queued: [], waitingFor: null, failedPreviews: 0 });
  });

  it('renders a preview for each imported game model without one', async () => {
    await openMockGame('crypt');
    const without = () =>
      useAppStore
        .getState()
        .workspace!.project!.assets!.filter(a => !a.thumbnail?.startsWith('thumbs/'));
    expect(without().length).toBeGreaterThan(0);
    render(<MoreMenu />);
    await openMore();
    await userEvent.click(screen.getByRole('menuitem', { name: /^Render \d+ previews?$/ }));
    await vi.waitFor(() => expect(without()).toEqual([]));
    await openMore();
    expect(screen.queryByRole('menuitem', { name: /^Render/ })).toBeNull();
  });

  it('is not offered while every model shows a render', async () => {
    withAssets([model('lantern', 'thumbs/lantern_v1.png')]);
    render(<MoreMenu />);
    await openMore();
    expect(screen.queryByRole('menuitem', { name: /^Render/ })).toBeNull();
  });

  it('counts the models without a render on a website and asks for them', async () => {
    withAssets([
      model('lantern', null),
      model('fox', 'refs/fox.png'),
      model('pine', 'thumbs/p.png'),
    ]);
    const asked = vi.spyOn(commands, 'thumbnailRenderMissing').mockResolvedValue({
      asked: 2,
      rendering: 2,
      renderingFiles: [
        '/ws/projects/frostveil/models/lantern_v1.glb',
        '/ws/projects/frostveil/models/fox_v1.glb',
      ],
      failed: 0,
      reason: null,
      project: useAppStore.getState().workspace!.project!,
    });
    render(<MoreMenu />);
    await openMore();
    await userEvent.click(screen.getByRole('menuitem', { name: 'Render 2 previews' }));
    expect(asked).toHaveBeenCalledWith('frostveil');
    // The renders are still queued in Blender, so the item waits for them.
    await openMore();
    await waitFor(() =>
      expect(screen.getByRole('menuitem', { name: 'Render 2 previews' })).toHaveAttribute(
        'aria-disabled',
        'true'
      )
    );
  });

  it('says when previews could not be queued', async () => {
    withAssets([model('lantern', null)]);
    vi.spyOn(commands, 'thumbnailRenderMissing').mockResolvedValue({
      asked: 1,
      rendering: 0,
      renderingFiles: [],
      failed: 1,
      reason: 'Blender is not installed.',
      project: useAppStore.getState().workspace!.project!,
    });
    render(<MoreMenu />);
    await openMore();
    await userEvent.click(screen.getByRole('menuitem', { name: 'Render 1 preview' }));
    await vi.waitFor(() =>
      expect(useAppStore.getState().notice).toMatchObject({
        status: 'failed',
        title: '1 preview could not render',
        detail: 'Blender is not installed. Render previews to retry.',
      })
    );
  });
});
