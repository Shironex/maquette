import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useInboxStore } from '@/state/use-inbox-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { BulkDiscardDialog } from '.';

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, undefined, 'triage');
  await useInboxStore.getState().load();
});

afterEach(() => {
  clearMocks();
  useInboxStore.setState({ snapshot: null, selected: null });
});

const pick = (...ids: string[]) =>
  useInboxStore.getState().snapshot!.items.filter(i => ids.includes(i.id));

describe('the bulk discard dialog', () => {
  it('names the models, says they are not deleted and which sends close, with Cancel focused', async () => {
    const onConfirm = vi.fn();
    render(
      <BulkDiscardDialog
        items={pick('crystal_lantern_redo.glb', 'observatory_arch_v3.glb')}
        busy={false}
        onConfirm={onConfirm}
        onClose={() => {}}
      />
    );
    const dialog = screen.getByRole('dialog', { name: 'Discard 2 models?' });
    expect(dialog).toHaveTextContent('Crystal lantern, Observatory arch leave the Inbox');
    expect(dialog).toHaveTextContent('not deleted');
    expect(dialog).toHaveTextContent('The 2 sends waiting on them are closed as cancelled');
    expect(screen.getByRole('button', { name: 'Cancel' })).toHaveFocus();
    await userEvent.click(screen.getByRole('button', { name: 'Discard 2' }));
    expect(onConfirm).toHaveBeenCalledOnce();
  });

  it('says nothing about sends for models nobody sent, and closes on Escape', async () => {
    const onClose = vi.fn();
    render(
      <BulkDiscardDialog
        items={pick('veil_platform_scan.glb')}
        busy={false}
        onConfirm={() => {}}
        onClose={onClose}
      />
    );
    expect(screen.getByRole('dialog')).not.toHaveTextContent('cancelled');
    await userEvent.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalledOnce();
  });
});
