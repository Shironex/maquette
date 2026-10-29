import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { InboxItem } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { useInboxStore } from '@/state/use-inbox-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import DiscardDialog from './DiscardDialog';

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, undefined, 'redo');
  await useInboxStore.getState().load();
});

afterEach(() => {
  clearMocks();
  useInboxStore.setState({ snapshot: null, selected: null });
  useAppStore.setState({ notice: null });
});

function inboxItem(id: string): InboxItem {
  const item = useInboxStore.getState().snapshot?.items.find(i => i.id === id);
  if (!item) throw new Error(`No mock inbox item ${id}`);
  return item;
}

describe('DiscardDialog', () => {
  it('asks first, with Cancel focused, and says the send closes', () => {
    render(<DiscardDialog item={inboxItem('veil_gate.glb')} onClose={() => {}} />);
    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveAccessibleName(/^Discard /);
    expect(dialog).toHaveTextContent('not deleted');
    expect(dialog).toHaveTextContent('closed as cancelled');
    expect(screen.getByRole('button', { name: 'Cancel' })).toHaveFocus();
    expect(useInboxStore.getState().snapshot?.items.some(i => i.id === 'veil_gate.glb')).toBe(true);
  });

  it('closes on Escape and discards nothing', async () => {
    const onClose = vi.fn();
    render(<DiscardDialog item={inboxItem('veil_gate.glb')} onClose={onClose} />);
    await userEvent.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalledOnce();
    expect(useInboxStore.getState().snapshot?.items.some(i => i.id === 'veil_gate.glb')).toBe(true);
  });

  it('discards the model and leaves the others', async () => {
    const onClose = vi.fn();
    render(<DiscardDialog item={inboxItem('rune_obelisk.glb')} onClose={onClose} />);
    expect(screen.getByRole('dialog')).not.toHaveTextContent('closed as cancelled');
    await userEvent.click(screen.getByRole('button', { name: 'Discard' }));
    await vi.waitFor(() => expect(onClose).toHaveBeenCalledOnce());
    const left = useInboxStore.getState().snapshot?.items.map(i => i.id) ?? [];
    expect(left).not.toContain('rune_obelisk.glb');
    expect(left).toContain('veil_gate.glb');
    expect(useAppStore.getState().notice?.status).toBe('done');
  });
});
