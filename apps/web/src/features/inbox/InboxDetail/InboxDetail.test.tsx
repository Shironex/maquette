import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { InboxItem } from '@/ipc/client';
import { useInboxStore } from '@/state/use-inbox-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import InboxDetail from './InboxDetail';
import { tooltipText } from '@/test/tooltipText';

vi.mock('@/features/viewer/ModelPreview', () => ({ default: () => null }));

beforeEach(async () => {
  await openMockWorkspace();
  await useInboxStore.getState().load();
});

afterEach(() => {
  clearMocks();
  useInboxStore.setState({ snapshot: null, selected: null });
  useProvidersStore.setState({ panel: null, jobs: [] });
});

function inboxItem(id: string): InboxItem {
  const item = useInboxStore.getState().snapshot?.items.find(i => i.id === id);
  if (!item) throw new Error(`No mock inbox item ${id}`);
  return item;
}

describe('InboxDetail', () => {
  it('lists the checks and the source of a model', () => {
    render(<InboxDetail item={inboxItem('character.glb')} />);
    const checks = screen.getByRole('complementary', { name: 'Automatic checks' });
    expect(checks).toHaveTextContent('4 passed · 3 to review');
    expect(checks).toHaveTextContent('Character');
    expect(
      screen.getByText('Review the 1 failure and 2 warnings before you accept it.')
    ).toBeInTheDocument();
  });

  it('offers Bones only for a rigged model', () => {
    const { unmount } = render(<InboxDetail item={inboxItem('character.glb')} />);
    const modes = screen.getByRole('group', { name: 'Preview mode' });
    expect(within(modes).getByRole('button', { name: 'Bones' })).toBeInTheDocument();
    unmount();
    render(<InboxDetail item={inboxItem('sakura_large.glb')} />);
    expect(screen.queryByRole('button', { name: 'Bones' })).toBeNull();
  });

  it('accepts a model into the open project', async () => {
    const accept = vi.fn(async () => true);
    useInboxStore.setState({ accept });
    render(<InboxDetail item={inboxItem('sakura_large.glb')} />);
    const button = screen.getByRole('button', { name: 'Accept to Blender' });
    expect(await tooltipText(button)).toBe('Moves it into Moon Island');
    await userEvent.click(button);
    expect(accept).toHaveBeenCalledWith('sakura_large.glb', 'moon-island');
  });

  it('says a linked model adds a version to its card', async () => {
    render(<InboxDetail item={inboxItem('flower_shrubs.glb')} />);
    expect(await tooltipText(screen.getByRole('button', { name: 'Accept to Blender' }))).toBe(
      'Adds a version to Flower shrubs'
    );
  });

  it('keeps a file it could not read out of the project', async () => {
    render(<InboxDetail item={inboxItem('far_spire.glb')} />);
    expect(screen.getByText('No preview for this file')).toBeInTheDocument();
    expect(screen.getByText('Could not read the file')).toBeInTheDocument();
    const accept = screen.getByRole('button', { name: 'Accept to Blender' });
    expect(accept).toHaveAttribute('aria-disabled', 'true');
    expect(await tooltipText(accept)).toBe('This file could not be read');
  });

  it('redoes a model that came from a send, and says why one without cannot', async () => {
    render(<InboxDetail item={inboxItem('flower_shrubs.glb')} />);
    await userEvent.click(screen.getByRole('button', { name: 'Redo...' }));
    expect(useProvidersStore.getState().panel).toMatchObject({
      asset: 'flower-shrubs',
      project: 'moon-island',
    });
  });

  it('keeps Redo off for a model that came from no send', async () => {
    render(<InboxDetail item={inboxItem('sakura_large.glb')} />);
    const redo = screen.getByRole('button', { name: 'Redo...' });
    expect(redo).toHaveAttribute('aria-disabled', 'true');
    expect(await tooltipText(redo)).toMatch(/did not come from a Send to 3D/);
  });

  it('notes a redo still on its way', () => {
    const shrubs = inboxItem('flower_shrubs.glb');
    useProvidersStore.setState({
      jobs: [
        {
          ...useProvidersStore.getState().jobs[0],
          id: 'flower-shrubs-2',
          asset: 'flower-shrubs',
          state: 'running',
          progress: 0.5,
          redoOf: 'flower-shrubs-1',
        } as never,
      ],
    });
    render(
      <InboxDetail item={{ ...shrubs, link: { ...shrubs.link!, redoOf: 'flower-shrubs-0' } }} />
    );
    const checks = screen.getByRole('complementary', { name: 'Automatic checks' });
    expect(checks).toHaveTextContent('Redo offlower-shrubs-0');
    expect(checks).toHaveTextContent('flower-shrubs-2, 50%');
  });
});
