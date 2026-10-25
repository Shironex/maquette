import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useInboxStore } from '@/state/use-inbox-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { tooltipText } from '@/test/tooltipText';
import { RowMenu } from '.';

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, undefined, 'redo');
  await useInboxStore.getState().load();
});

afterEach(() => {
  clearMocks();
  useInboxStore.setState({ snapshot: null, selected: null, checked: [] });
  useProvidersStore.setState({ panel: null, batch: null });
});

const item = (id: string) => useInboxStore.getState().snapshot!.items.find(i => i.id === id)!;

describe('an inbox row menu', () => {
  it('redoes a model from its send', async () => {
    const onClose = vi.fn();
    render(<RowMenu item={item('sakura_large.glb')} onClose={onClose} />);
    await userEvent.click(screen.getByRole('menuitem', { name: 'Redo...' }));
    expect(onClose).toHaveBeenCalled();
    expect(useProvidersStore.getState().panel).toMatchObject({
      project: 'moon-island',
      asset: 'sakura',
    });
  });

  it('says why a model that came from no send cannot be redone', async () => {
    render(<RowMenu item={item('far_spire.glb')} onClose={() => {}} />);
    const redo = screen.getByRole('menuitem', { name: 'Redo...' });
    expect(redo).toHaveAttribute('aria-disabled', 'true');
    expect(await tooltipText(redo)).toMatch(/did not come from a Send to 3D/);
  });

  it('redoes the ticked models as one batch', async () => {
    useInboxStore.getState().check('character.glb', true);
    useInboxStore.getState().check('sakura_large.glb', true);
    render(<RowMenu item={item('character.glb')} onClose={() => {}} />);
    expect(screen.getByRole('menuitem', { name: 'Deselect' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('menuitem', { name: 'Redo 2 selected...' }));
    expect(useProvidersStore.getState().batch).toMatchObject({
      assets: ['character', 'sakura'],
      redo: true,
    });
    expect(useInboxStore.getState().checked).toEqual([]);
  });
});
