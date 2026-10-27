import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { StartersBanner } from '.';

let unsubscribe = () => {};

/** Renders the banner for the store's view and follows the view as the store updates it. */
function renderBanner() {
  unsubscribe();
  const view = usePipelineStore.getState().view!;
  const result = render(<StartersBanner view={view} busy={false} />);
  unsubscribe = usePipelineStore.subscribe(s => {
    if (s.view) result.rerender(<StartersBanner view={s.view} busy={false} />);
  });
  return result;
}

afterEach(() => {
  unsubscribe();
  clearMocks();
  usePipelineStore.setState({ view: null });
  localStorage.clear();
});

describe('the starter scripts banner', () => {
  it('updates the untouched old starters in one click and leaves the edited one', async () => {
    await openMockWorkspace();
    await usePipelineStore.getState().load('frostveil');
    renderBanner();
    // The banner names the project it updates: Update acts on the open one.
    expect(screen.getByText('New starter scripts for Frostveil')).toBeInTheDocument();
    expect(
      screen.getByText(
        /build_scene\.py, bake_light\.py and render_stops\.py in Frostveil's tools folder are untouched copies/
      )
    ).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Update' }));
    await waitFor(() => expect(screen.queryByText('New starter scripts for Frostveil')).toBeNull());
    expect(useAppStore.getState().notice).toMatchObject({
      status: 'done',
      title: '3 starter scripts updated in Frostveil',
      detail: 'Now current: build_scene.py, bake_light.py and render_stops.py.',
    });
    const starters = usePipelineStore.getState().view!.starters;
    expect(starters.find(s => s.path === 'tools/bake_light.py')?.state).toBe('current');
    expect(starters.find(s => s.path === 'tools/export_web.py')?.state).toBe('edited');
    expect(screen.getByText('export_web.py in Frostveil was edited')).toBeInTheDocument();
  });

  it('replaces an edited script only when asked, or keeps it', async () => {
    await openMockWorkspace();
    await usePipelineStore.getState().load('frostveil');
    const { unmount } = renderBanner();
    await userEvent.click(screen.getByRole('button', { name: 'Keep mine' }));
    expect(screen.queryByText('export_web.py in Frostveil was edited')).toBeNull();
    unmount();
    localStorage.clear();
    renderBanner();
    await userEvent.click(screen.getByRole('button', { name: 'Replace with the new starter' }));
    await waitFor(() =>
      expect(
        usePipelineStore.getState().view!.starters.find(s => s.path === 'tools/export_web.py')
      ).toMatchObject({ state: 'current', backup: 'tools/export_web.py.bak' })
    );
  });

  it('says so when the update is refused instead of passing quietly', async () => {
    await openMockWorkspace();
    await usePipelineStore.getState().load('frostveil');
    const ok = await usePipelineStore.getState().updateStarters(['tools/bake.py'], false);
    expect(ok).toBe(false);
    expect(useAppStore.getState().notice).toMatchObject({
      status: 'failed',
      title: 'The scripts in Frostveil were not updated',
      detail: "tools/bake.py is not a starter script of this project's template.",
    });
    expect(await usePipelineStore.getState().updateStarters([], false)).toBe(false);
    expect(useAppStore.getState().notice?.status).toBe('failed');
  });

  it('shows nothing for a project with current starters', async () => {
    await openMockWorkspace();
    await usePipelineStore.getState().load('frost-cathedral');
    const { container } = renderBanner();
    expect(container).toBeEmptyDOMElement();
  });
});
