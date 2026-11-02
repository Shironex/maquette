import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { clearMocks, openPlans } from '@/test/open-plan';
import { useAppStore } from './use-app-store';
import { SAVE_DELAY, usePlansStore } from './use-plans-store';

beforeEach(() => vi.useFakeTimers({ shouldAdvanceTime: true }));
afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
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

describe('leaving a canvas', () => {
  const edit = (text: string) => usePlansStore.getState().updateNode('n-01', { text });
  const savedText = () =>
    usePlansStore.getState().open?.plan.nodes.find(n => n.id === 'n-01')?.text;

  it('keeps the canvas open when the save on the way out fails, and asks', async () => {
    await openPlans();
    await usePlansStore.getState().openCanvas('combat-ideas');
    const save = vi
      .spyOn(commands, 'planSave')
      .mockRejectedValueOnce({ code: 'io', message: 'The file is locked' });
    edit('Kept even when the disk says no');
    expect(await usePlansStore.getState().close()).toBe(false);
    let s = usePlansStore.getState();
    expect(s.open?.id).toBe('combat-ideas');
    expect(s.leaving).toEqual({ to: 'list' });
    expect(s.dirty).toBe(true);
    expect(s.nodes.find(n => n.id === 'n-01')?.data.node.text).toBe(
      'Kept even when the disk says no'
    );

    // Try again saves the edit and then goes back to the list.
    await usePlansStore.getState().retryLeave();
    s = usePlansStore.getState();
    expect(save).toHaveBeenCalledTimes(2);
    expect(s.open).toBeNull();
    expect(s.leaving).toBeNull();
    expect(s.list?.find(p => p.id === 'combat-ideas')).toBeDefined();
  });

  it('leaves without saving only when asked to', async () => {
    await openPlans();
    await usePlansStore.getState().openCanvas('combat-ideas');
    vi.spyOn(commands, 'planSave').mockRejectedValue({ code: 'io', message: 'Gone' });
    edit('Dropped on purpose');
    expect(await usePlansStore.getState().close()).toBe(false);
    usePlansStore.getState().stay();
    expect(usePlansStore.getState().open?.id).toBe('combat-ideas');
    expect(await usePlansStore.getState().close()).toBe(false);
    await usePlansStore.getState().leaveAnyway();
    expect(usePlansStore.getState().open).toBeNull();
    expect(usePlansStore.getState().dirty).toBe(false);
  });

  it('waits for a save on its way and then saves the edits made during it', async () => {
    await openPlans();
    await usePlansStore.getState().openCanvas('combat-ideas');
    const real = commands.planSave.bind(commands);
    let release: () => void = () => undefined;
    const held = new Promise<void>(done => (release = done));
    const save = vi.spyOn(commands, 'planSave').mockImplementationOnce(async (...args) => {
      await held;
      return real(...args);
    });
    edit('First');
    const first = usePlansStore.getState().save();
    edit('Second, while the first saves');
    const closing = usePlansStore.getState().close();
    release();
    expect(await first).toBe(true);
    expect(await closing).toBe(true);
    expect(save).toHaveBeenCalledTimes(2);
    expect(save.mock.calls[1]?.[2].nodes.find(n => n.id === 'n-01')?.text).toBe(
      'Second, while the first saves'
    );
    expect(usePlansStore.getState().open).toBeNull();
  });

  it('saves pending edits before another project, and asks when that fails', async () => {
    await openPlans();
    await usePlansStore.getState().openCanvas('combat-ideas');
    const save = vi.spyOn(commands, 'planSave');
    edit('Before the switch');
    await usePlansStore.getState().loadList('moon-island');
    expect(save).toHaveBeenCalledOnce();
    expect(usePlansStore.getState().project).toBe('moon-island');

    await usePlansStore.getState().loadList('frostveil');
    await usePlansStore.getState().openCanvas('combat-ideas');
    expect(savedText()).toBe('Before the switch');
    save.mockRejectedValueOnce({ code: 'io', message: 'Locked' });
    edit('Not lost');
    await usePlansStore.getState().loadList('moon-island');
    const s = usePlansStore.getState();
    expect(s.project).toBe('frostveil');
    expect(s.leaving).toEqual({ to: 'project', project: 'moon-island' });
    await usePlansStore.getState().retryLeave();
    expect(usePlansStore.getState().project).toBe('moon-island');
  });

  it('saves everything for quitting, or says why not', async () => {
    await openPlans();
    await usePlansStore.getState().openCanvas('combat-ideas');
    edit('For the quit');
    expect(await usePlansStore.getState().saveForQuit()).toBeNull();
    expect(savedText()).toBe('For the quit');
    vi.spyOn(commands, 'planSave').mockRejectedValueOnce({ code: 'io', message: 'Disk full' });
    edit('Not saved');
    expect(await usePlansStore.getState().saveForQuit()).toBe('Disk full');
  });
});

describe('saves that settle', () => {
  it('opens a file with untrimmed fields and leaves without saving again and again', async () => {
    await openPlans();
    const real = await commands.planGet('frostveil', 'combat-ideas');
    const loose = {
      ...real,
      plan: {
        ...real.plan,
        nodes: real.plan.nodes.map(n =>
          n.id === 'n-01'
            ? { ...n, title: ' Frost veil ' }
            : n.id === 'n-05'
              ? { ...n, url: ` ${n.url} ` }
              : n
        ),
        edges: real.plan.edges.map(e => (e.id === 'e-01' ? { ...e, label: '' } : e)),
      },
    };
    vi.spyOn(commands, 'planGet').mockResolvedValueOnce(loose);
    const save = vi.spyOn(commands, 'planSave');
    await usePlansStore.getState().openCanvas('combat-ideas');
    await vi.advanceTimersByTimeAsync(SAVE_DELAY * 6);
    expect(save.mock.calls.length).toBeLessThanOrEqual(1);
    expect(await usePlansStore.getState().close()).toBe(true);
    expect(save.mock.calls.length).toBeLessThanOrEqual(1);
    expect(usePlansStore.getState().dirty).toBe(false);
  });

  it('saves a picture captioned with spaces once', async () => {
    await openPlans();
    await usePlansStore.getState().openCanvas('combat-ideas');
    const save = vi.spyOn(commands, 'planSave');
    usePlansStore
      .getState()
      .addNode('image', { x: 0, y: 900 }, { image: 'concepts/porch .png', title: 'porch ' });
    await vi.advanceTimersByTimeAsync(SAVE_DELAY * 6);
    expect(save).toHaveBeenCalledOnce();
    expect(usePlansStore.getState().dirty).toBe(false);
    expect(await usePlansStore.getState().close()).toBe(true);
    expect(save).toHaveBeenCalledOnce();
  });

  it('shows the core canvas when the core changed what was sent', async () => {
    await openPlans();
    await usePlansStore.getState().openCanvas('combat-ideas');
    const real = commands.planSave.bind(commands);
    vi.spyOn(commands, 'planSave').mockImplementationOnce(async (project, id, plan) => {
      const doc = await real(project, id, plan);
      return { ...doc, plan: { ...doc.plan, nodes: doc.plan.nodes.filter(n => n.id !== 'n-06') } };
    });
    usePlansStore.getState().updateNode('n-01', { text: 'Changed' });
    await usePlansStore.getState().save();
    expect(usePlansStore.getState().nodes.map(n => n.id)).not.toContain('n-06');
    expect(usePlansStore.getState().dirty).toBe(false);
  });

  it('keeps the new name when a save from before a rename answers after it', async () => {
    await openPlans();
    await usePlansStore.getState().openCanvas('boss-arena');
    const real = commands.planSave.bind(commands);
    let release: () => void = () => undefined;
    const held = new Promise<void>(done => (release = done));
    vi.spyOn(commands, 'planSave').mockImplementationOnce(async (project, id, plan) => {
      const doc = await real(project, id, plan);
      await held;
      return { ...doc, plan: { ...doc.plan, name: 'Boss arena' } };
    });
    usePlansStore.getState().addNode('note', { x: 0, y: 0 });
    const saving = usePlansStore.getState().save();
    await vi.waitFor(() => expect(usePlansStore.getState().saving).toBe(true));
    vi.spyOn(commands, 'planRename').mockResolvedValueOnce({
      ...usePlansStore.getState().list!.find(p => p.id === 'boss-arena')!,
      name: 'Warden arena',
    });
    // The rename's own save waits for the held one, so answer that first.
    const renaming = usePlansStore.getState().rename('boss-arena', 'Warden arena');
    release();
    await saving;
    await renaming;
    expect(usePlansStore.getState().open?.plan.name).toBe('Warden arena');
  });

  it('goes back to the workspace project after a failed switch and Stay', async () => {
    await openPlans();
    await usePlansStore.getState().openCanvas('combat-ideas');
    vi.spyOn(commands, 'planSave').mockRejectedValueOnce({ code: 'io', message: 'Locked' });
    usePlansStore.getState().updateNode('n-01', { text: 'Kept' });
    useAppStore.setState(s => ({
      workspace: s.workspace && {
        ...s.workspace,
        project: { ...s.workspace.project!, id: 'moon-island' },
      },
    }));
    await usePlansStore.getState().loadList('moon-island');
    usePlansStore.getState().stay();
    expect(usePlansStore.getState().project).toBe('frostveil');
    expect(await usePlansStore.getState().close()).toBe(true);
    expect(usePlansStore.getState().project).toBe('moon-island');
    expect(usePlansStore.getState().list).toEqual([]);
  });
});
