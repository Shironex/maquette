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
    render(<InboxDetail item={inboxItem('elsheria.glb')} />);
    const checks = screen.getByRole('complementary', { name: 'Automatic checks' });
    expect(checks).toHaveTextContent('5 passed · 3 to review');
    expect(checks).toHaveTextContent('Character');
    expect(
      screen.getByText('Review the 1 failure and 2 warnings before you accept it.')
    ).toBeInTheDocument();
  });

  it('stops and starts the turntable from a pressed button', async () => {
    render(<InboxDetail item={inboxItem('elsheria.glb')} />);
    const spin = screen.getByRole('button', { name: 'Stop rotating', pressed: true });
    await userEvent.click(spin);
    expect(screen.getByRole('button', { name: 'Rotate the view', pressed: false })).toBe(spin);
  });

  it('offers Bones only for a rigged model', () => {
    const { unmount } = render(<InboxDetail item={inboxItem('elsheria.glb')} />);
    const modes = screen.getByRole('group', { name: 'Preview mode' });
    expect(within(modes).getByRole('button', { name: 'Bones' })).toBeInTheDocument();
    unmount();
    render(<InboxDetail item={inboxItem('veil_gate.glb')} />);
    expect(screen.queryByRole('button', { name: 'Bones' })).toBeNull();
  });

  it('accepts a model into the open project', async () => {
    const accept = vi.fn(async () => true);
    useInboxStore.setState({ accept });
    render(<InboxDetail item={inboxItem('veil_gate.glb')} />);
    const button = screen.getByRole('button', { name: 'Accept to Blender' });
    expect(await tooltipText(button)).toBe('Moves it into Frostveil');
    await userEvent.click(button);
    expect(accept).toHaveBeenCalledWith('veil_gate.glb', 'frostveil');
  });

  it('says a linked model adds a version to its card', async () => {
    render(<InboxDetail item={inboxItem('ice_crystals.glb')} />);
    expect(await tooltipText(screen.getByRole('button', { name: 'Accept to Blender' }))).toBe(
      'Adds a version to Ice crystals'
    );
  });

  it('keeps a file it could not read out of the project', async () => {
    render(<InboxDetail item={inboxItem('rune_obelisk.glb')} />);
    expect(screen.getByText('No preview for this file')).toBeInTheDocument();
    expect(screen.getByText('Could not read the file')).toBeInTheDocument();
    const accept = screen.getByRole('button', { name: 'Accept to Blender' });
    expect(accept).toHaveAttribute('aria-disabled', 'true');
    expect(await tooltipText(accept)).toBe('This file could not be read');
  });

  it('redoes a model that came from a send, and says why one without cannot', async () => {
    render(<InboxDetail item={inboxItem('ice_crystals.glb')} />);
    await userEvent.click(screen.getByRole('button', { name: 'Redo...' }));
    expect(useProvidersStore.getState().panel).toMatchObject({
      asset: 'ice-crystals',
      project: 'frostveil',
    });
  });

  it('keeps Redo off for a model that came from no send', async () => {
    render(<InboxDetail item={inboxItem('veil_gate.glb')} />);
    const redo = screen.getByRole('button', { name: 'Redo...' });
    expect(redo).toHaveAttribute('aria-disabled', 'true');
    expect(await tooltipText(redo)).toMatch(/did not come from a Send to 3D/);
  });

  it('notes a redo still on its way', () => {
    const shrubs = inboxItem('ice_crystals.glb');
    useProvidersStore.setState({
      jobs: [
        {
          ...useProvidersStore.getState().jobs[0],
          id: 'ice-crystals-2',
          asset: 'ice-crystals',
          state: 'running',
          progress: 0.5,
          redoOf: 'ice-crystals-1',
        } as never,
      ],
    });
    render(
      <InboxDetail item={{ ...shrubs, link: { ...shrubs.link!, redoOf: 'ice-crystals-0' } }} />
    );
    const checks = screen.getByRole('complementary', { name: 'Automatic checks' });
    expect(checks).toHaveTextContent('Redo ofice-crystals-0');
    expect(checks).toHaveTextContent('ice-crystals-2, 50%');
  });
});
