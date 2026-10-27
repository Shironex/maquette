import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { forgetStoredPrefs, useBoardPrefsStore } from '@/state/use-board-prefs-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useAppStore } from '@/state/use-app-store';
import { commands, type Asset, type ProjectManifest } from '@/ipc/client';
import { useGameStore } from '@/state/use-game-store';
import { clearMocks, openMockGame, openMockWorkspace } from '@/test/mock-ipc';
import BoardHeader from './BoardHeader';

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, 'batch');
  await useProvidersStore.getState().load('moon-island');
});

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ project: null, jobs: [], queueOpen: false, busy: [] });
  window.localStorage.clear();
  forgetStoredPrefs();
  useBoardPrefsStore.setState({ byProject: {} });
});

describe('BoardHeader', () => {
  it('gives every button of the header a tooltip', async () => {
    await openMockGame('crypt');
    render(<BoardHeader queue={() => null} />);
    const buttons = screen.getAllByRole('button');
    const names = buttons.map(b => b.textContent ?? '');
    for (const name of ['By hand', '3D queue', 'Import models', 'Built asset', 'Add asset'])
      expect(
        names.some(n => n.startsWith(name)),
        name
      ).toBe(true);
    for (const button of buttons) {
      await userEvent.hover(button);
      expect(await screen.findByRole('tooltip'), button.textContent ?? '').toBeVisible();
      await userEvent.unhover(button);
      await waitFor(() => expect(screen.queryByRole('tooltip')).toBeNull());
    }
  });

  it('says what By hand opens', async () => {
    render(<BoardHeader queue={() => null} />);
    await userEvent.hover(screen.getByRole('button', { name: /By hand/ }));
    expect(await screen.findByRole('tooltip')).toHaveTextContent(
      "By hand: models waiting for you to make on the provider's site"
    );
  });

  it('renders a preview for each imported game model without one', async () => {
    await openMockGame('crypt');
    const without = () =>
      useAppStore
        .getState()
        .workspace!.project!.assets!.filter(a => !a.thumbnail?.startsWith('thumbs/'));
    expect(without().length).toBeGreaterThan(0);
    render(<BoardHeader queue={() => null} />);
    await userEvent.click(screen.getByRole('button', { name: /^Render \d+ previews?$/ }));
    await vi.waitFor(() => expect(without()).toEqual([]));
    expect(screen.queryByRole('button', { name: /^Render \d+ previews?$/ })).toBeNull();
  });

  describe('Render previews on a website project', () => {
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
      useGameStore.setState({ rendering: false, waiting: 0, waitingFor: null, failedPreviews: 0 });
    });

    it('is hidden while every model shows a render', () => {
      withAssets([model('lantern', 'thumbs/lantern_v1.png')]);
      render(<BoardHeader queue={() => null} />);
      expect(screen.queryByRole('button', { name: /^Render \d+ previews?$/ })).toBeNull();
    });

    it('counts the models without a render and asks for them', async () => {
      withAssets([
        model('lantern', null),
        model('fox', 'refs/fox.png'),
        model('pine', 'thumbs/p.png'),
      ]);
      const asked = vi.spyOn(commands, 'thumbnailRenderMissing').mockResolvedValue({
        asked: 2,
        rendering: 2,
        failed: 0,
        reason: null,
        project: useAppStore.getState().workspace!.project!,
      });
      render(<BoardHeader queue={() => null} />);
      const button = screen.getByRole('button', { name: 'Render 2 previews' });
      await userEvent.click(button);
      expect(asked).toHaveBeenCalledWith('moon-island');
      // The renders are still queued in Blender, so the button waits for them.
      await waitFor(() => expect(button).toHaveAttribute('aria-busy', 'true'));
    });

    it('says when previews could not be queued', async () => {
      withAssets([model('lantern', null)]);
      vi.spyOn(commands, 'thumbnailRenderMissing').mockResolvedValue({
        asked: 1,
        rendering: 0,
        failed: 1,
        reason: 'Blender is not installed.',
        project: useAppStore.getState().workspace!.project!,
      });
      render(<BoardHeader queue={() => null} />);
      await userEvent.click(screen.getByRole('button', { name: 'Render 1 preview' }));
      await vi.waitFor(() =>
        expect(useAppStore.getState().notice).toMatchObject({
          status: 'failed',
          title: '1 preview could not render',
          detail: 'Blender is not installed. Render previews to retry.',
        })
      );
    });
  });

  it('names the project and counts the By hand models still to make', () => {
    render(<BoardHeader queue={() => null} />);
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('Moon Island');
    expect(screen.getByRole('button', { name: /By hand/ })).toHaveTextContent('By hand5');
  });

  it('opens the 3D queue the shell supplies', async () => {
    const queue = vi.fn(() => <div role="dialog" aria-label="3D queue panel" />);
    render(<BoardHeader queue={queue} />);
    expect(queue).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole('button', { name: /3D queue/ }));
    expect(screen.getByRole('dialog', { name: '3D queue panel' })).toBeInTheDocument();
    expect(useProvidersStore.getState().queueOpen).toBe(true);
  });

  it('toggles the add asset form', async () => {
    render(<BoardHeader queue={() => null} />);
    const add = screen.getByRole('button', { name: 'Add asset' });
    await userEvent.click(add);
    expect(screen.getByRole('form', { name: 'Add asset' })).toBeInTheDocument();
    expect(add).toHaveAttribute('aria-expanded', 'true');
    await userEvent.click(add);
    expect(screen.queryByRole('form', { name: 'Add asset' })).toBeNull();
  });

  it('switches the Board to the list and remembers it for the project', async () => {
    render(<BoardHeader queue={() => null} />);
    const layout = screen.getByRole('group', { name: 'Board layout' });
    const list = within(layout).getByRole('button', { name: 'List' });
    expect(within(layout).getByRole('button', { name: 'Board' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
    await userEvent.click(list);
    expect(list).toHaveAttribute('aria-pressed', 'true');
    expect(window.localStorage.getItem('maquette.board.moon-island')).toContain('"list"');
  });
});
