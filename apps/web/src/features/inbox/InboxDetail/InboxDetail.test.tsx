import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { InboxItem } from '@/ipc/client';
import { useInboxStore } from '@/state/use-inbox-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import InboxDetail from './InboxDetail';

vi.mock('@/features/viewer/ModelPreview', () => ({ default: () => null }));

beforeEach(async () => {
  await openMockWorkspace();
  await useInboxStore.getState().load();
});

afterEach(() => {
  clearMocks();
  useInboxStore.setState({ snapshot: null, selected: null });
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
    expect(button).toHaveAttribute('title', 'Moves it into Moon Island');
    await userEvent.click(button);
    expect(accept).toHaveBeenCalledWith('sakura_large.glb', 'moon-island');
  });

  it('says a linked model adds a version to its card', () => {
    render(<InboxDetail item={inboxItem('flower_shrubs.glb')} />);
    expect(screen.getByRole('button', { name: 'Accept to Blender' })).toHaveAttribute(
      'title',
      'Adds a version to Flower shrubs'
    );
  });

  it('keeps a file it could not read out of the project', () => {
    render(<InboxDetail item={inboxItem('far_spire.glb')} />);
    expect(screen.getByText('No preview for this file')).toBeInTheDocument();
    expect(screen.getByText('Could not read the file')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Accept to Blender' })).toBeDisabled();
  });
});
