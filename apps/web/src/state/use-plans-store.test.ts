import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { clearMocks, openPlans } from '@/test/open-plan';
import { SAVE_DELAY, usePlansStore } from './use-plans-store';

beforeEach(() => vi.useFakeTimers({ shouldAdvanceTime: true }));
afterEach(() => {
  vi.useRealTimers();
  clearMocks();
});

describe('the plans store', () => {
  it('lists the project canvases and opens one', async () => {
    const store = await openPlans();
    expect(store.list?.map(p => p.id)).toEqual(['boss-arena', 'combat-ideas', 'opening-story']);
    await usePlansStore.getState().openCanvas('combat-ideas');
    const s = usePlansStore.getState();
    expect(s.open?.plan.name).toBe('Combat ideas');
    expect(s.nodes).toHaveLength(6);
    expect(s.edges).toHaveLength(3);
    expect(Object.keys(s.images)).toContain('concepts/__pool/veil-gate-kit-b.webp');
  });

  it('saves a moment after a move, and not for a selection', async () => {
    await openPlans();
    await usePlansStore.getState().openCanvas('combat-ideas');
    const save = vi.spyOn(commands, 'planSave');
    usePlansStore.getState().onNodesChange([{ type: 'select', id: 'n-01', selected: true }]);
    await vi.advanceTimersByTimeAsync(SAVE_DELAY * 2);
    expect(save).not.toHaveBeenCalled();

    usePlansStore
      .getState()
      .onNodesChange([
        { type: 'position', id: 'n-01', position: { x: 90.4, y: 80 }, dragging: false },
      ]);
    await vi.advanceTimersByTimeAsync(SAVE_DELAY + 50);
    expect(save).toHaveBeenCalledTimes(1);
    const sent = save.mock.calls[0]?.[2];
    expect(sent?.nodes.find(n => n.id === 'n-01')?.x).toBe(90);
    expect(usePlansStore.getState().open?.plan.nodes.find(n => n.id === 'n-01')?.x).toBe(90);
  });

  it('adds notes, links and pictures, links two nodes once and deletes', async () => {
    await openPlans(null);
    await usePlansStore.getState().create('Lantern festival');
    const s = usePlansStore.getState();
    expect(s.open?.id).toBe('lantern-festival');
    const a = s.addNode('note', { x: 0, y: 0 });
    expect(usePlansStore.getState().fresh).toBe(a);
    const b = usePlansStore
      .getState()
      .addNode('link', { x: 300, y: 0 }, { url: 'https://a.example' });
    expect(await usePlansStore.getState().addPicture({ x: 0, y: 300 })).toBe(true);
    const connect = { source: a, target: b, sourceHandle: 'right', targetHandle: 'left' };
    usePlansStore.getState().onConnect(connect);
    usePlansStore.getState().onConnect({ ...connect, source: b, target: a });
    expect(usePlansStore.getState().edges).toHaveLength(1);
    usePlansStore.getState().updateNode(a, { text: 'Lanterns float up' });
    await usePlansStore.getState().save();
    const saved = usePlansStore.getState().open?.plan;
    expect(saved?.nodes).toHaveLength(3);
    expect(saved?.nodes.find(n => n.id === a)?.text).toBe('Lanterns float up');
    expect(saved?.edges).toHaveLength(1);

    usePlansStore.getState().onNodesChange([{ type: 'remove', id: b }]);
    usePlansStore.getState().onEdgesChange([{ type: 'remove', id: saved!.edges[0]!.id }]);
    await usePlansStore.getState().save();
    expect(usePlansStore.getState().open?.plan.nodes).toHaveLength(2);
    expect(usePlansStore.getState().open?.plan.edges).toHaveLength(0);
  });

  it('renames and removes a canvas, and says when a save fails', async () => {
    await openPlans();
    expect(await usePlansStore.getState().rename('boss-arena', 'Warden arena')).toBe(true);
    expect(usePlansStore.getState().list?.find(p => p.id === 'boss-arena')?.name).toBe(
      'Warden arena'
    );
    expect(await usePlansStore.getState().rename('boss-arena', '  ')).toBe(false);

    await usePlansStore.getState().openCanvas('opening-story');
    expect(await usePlansStore.getState().remove('opening-story')).toBe(true);
    expect(usePlansStore.getState().open).toBeNull();
    expect(usePlansStore.getState().list?.map(p => p.id)).toEqual(['boss-arena', 'combat-ideas']);

    await usePlansStore.getState().openCanvas('combat-ideas');
    vi.spyOn(commands, 'planSave').mockRejectedValueOnce({ code: 'io', message: 'Disk full' });
    usePlansStore.getState().addNode('note', { x: 0, y: 0 });
    expect(await usePlansStore.getState().save()).toBe(false);
    expect(usePlansStore.getState().saveError).toBe('Disk full');
  });
});
