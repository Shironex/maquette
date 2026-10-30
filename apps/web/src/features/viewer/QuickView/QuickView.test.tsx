import { act, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { registerMockFile } from '@/ipc/files';
import type { QuickViewTarget } from '@/state/quick-view';
import { useAppStore } from '@/state/use-app-store';
import { useInspectorStore } from '@/state/use-inspector-store';
import { useQuickViewStore } from '@/state/use-quick-view-store';
import { clearMocks } from '@/test/mock-ipc';
import { tooltipText } from '@/test/tooltipText';
import QuickView from './QuickView';

vi.mock('@/features/viewer/ModelPreview', () => ({
  default: ({ url, mode }: { url: string; mode: string }) => (
    <div data-testid="preview" data-url={url} data-mode={mode} />
  ),
}));

const LANTERN: QuickViewTarget = {
  name: 'Crystal lantern',
  version: 2,
  file: '/mock/projects/frostveil/models/crystal_lantern_v2.glb',
  stats: {
    triangles: 12_900,
    bytes: 400_000,
    textures: 3,
    maxTexture: 2048,
    joints: 0,
    animations: 0,
  },
  from: 'Frostveil',
  inspect: { project: 'frostveil', asset: 'crystal-lantern', version: 2 },
  inspectBlocked: null,
};

afterEach(() => {
  clearMocks();
  useQuickViewStore.setState({ target: null });
});

describe('the 3D quick view', () => {
  it('shows the model with its version, triangles, size and preview modes', async () => {
    registerMockFile(LANTERN.file, '/mock/frost_lantern.glb');
    useQuickViewStore.setState({ target: LANTERN });
    render(<QuickView />);
    const dialog = screen.getByRole('dialog', { name: 'Crystal lantern' });
    expect(dialog).toHaveAccessibleDescription('v2 · 12.9k triangles · 400 KB · Frostveil');
    expect(await within(dialog).findByTestId('preview')).toHaveAttribute(
      'data-url',
      '/mock/frost_lantern.glb'
    );
    expect(within(dialog).getByRole('button', { name: 'Close' })).toHaveFocus();
    const modes = within(dialog).getByRole('group', { name: 'Preview mode' });
    expect(
      within(modes)
        .getAllByRole('button')
        .map(b => b.textContent)
    ).toEqual(['Shaded', 'Wireframe', 'UV']);
    await userEvent.click(within(modes).getByRole('button', { name: 'Wireframe' }));
    expect(within(dialog).getByTestId('preview')).toHaveAttribute('data-mode', 'wireframe');
  });

  it('closes with Escape and on a click outside it, unmounting the preview', async () => {
    registerMockFile(LANTERN.file, '/mock/frost_lantern.glb');
    useQuickViewStore.setState({ target: LANTERN });
    render(<QuickView />);
    await screen.findByTestId('preview');
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(screen.queryByTestId('preview')).toBeNull();
    act(() => useQuickViewStore.setState({ target: LANTERN }));
    await userEvent.click(screen.getByRole('dialog').parentElement!);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('opens the version in the Inspector', async () => {
    registerMockFile(LANTERN.file, '/mock/frost_lantern.glb');
    useQuickViewStore.setState({ target: LANTERN });
    render(<QuickView />);
    await screen.findByTestId('preview');
    await userEvent.click(screen.getByRole('button', { name: 'Open in Inspector' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(useAppStore.getState().view).toBe('inspector');
    expect(useInspectorStore.getState()).toMatchObject({
      tab: 'compare',
      project: 'frostveil',
      comparison: 'asset/crystal-lantern',
      right: 'asset/crystal-lantern/v2/side',
      selected: 'asset/crystal-lantern/v2',
    });
    useAppStore.setState({ view: 'board' });
  });

  it('says why Open in Inspector is off for a library model the project lacks', async () => {
    useQuickViewStore.setState({
      target: {
        ...LANTERN,
        inspect: null,
        inspectBlocked: 'Add it to Frostveil to inspect it in full',
      },
    });
    render(<QuickView />);
    const open = screen.getByRole('button', { name: 'Open in Inspector' });
    expect(open).toHaveAttribute('aria-disabled', 'true');
    expect(await tooltipText(open)).toBe('Add it to Frostveil to inspect it in full');
  });
});
