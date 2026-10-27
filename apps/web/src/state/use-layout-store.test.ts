import { afterEach, describe, expect, it } from 'vitest';
import { commands } from '@/ipc/client';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useLayoutStore } from './use-layout-store';

const EMPTY = { models: {}, stops: [], desk: null };

afterEach(() => {
  clearMocks();
  useLayoutStore.setState({
    view: null,
    project: null,
    saved: EMPTY,
    draft: EMPTY,
    history: { past: [], future: [] },
    outside: false,
    selection: null,
    preview: null,
    pose: null,
  });
});

const store = () => useLayoutStore.getState();

describe('the layout store', () => {
  it('loads the layout and undoes and redoes a placement', async () => {
    await openMockWorkspace();
    await store().load('frostveil');
    expect(store().draft.stops.map(s => s.name)).toEqual([
      'Arrival',
      'Veil gate',
      'Crystals',
      'Keystone',
      'Pullback',
    ]);
    const before = store().draft.models.elsheria!;
    store().placeModel('elsheria', { ...before, position: [0, 0, 0] });
    expect(store().draft.models.elsheria!.position).toEqual([0, 0, 0]);
    store().undo();
    expect(store().draft.models.elsheria).toEqual(before);
    store().redo();
    expect(store().draft.models.elsheria!.position).toEqual([0, 0, 0]);
  });

  it('counts a whole gizmo drag as one undo step', async () => {
    await openMockWorkspace();
    await store().load('frostveil');
    const before = store().draft.models['crystal-lantern']!;
    store().beginDrag();
    for (const x of [1, 2, 3])
      store().dragModel('crystal-lantern', { ...before, position: [x, 0, 0] });
    expect(store().history.past).toHaveLength(1);
    store().undo();
    expect(store().draft.models['crystal-lantern']).toEqual(before);
  });

  it('adds a stop where the editor camera stands and selects it', async () => {
    await openMockWorkspace();
    await store().load('frostveil');
    store().addStopHere();
    expect(store().draft.stops).toHaveLength(5);
    store().setPose({ position: [1, 2, 3], target: [0, 1, 0], fov: 50 });
    store().addStopHere();
    const added = store().draft.stops[5]!;
    expect(added).toMatchObject({ name: 'Stop 6', position: [1, 2, 3], fov: 50 });
    expect(store().selection).toEqual({ kind: 'stop', id: added.id });
    store().removeStop(added.id);
    expect(store().selection).toBeNull();
  });

  it('saves every model pinned and keeps the undo steps', async () => {
    await openMockWorkspace();
    await store().load('frost-cathedral');
    const models = store().view!.models;
    expect(models.length).toBeGreaterThan(0);
    expect(store().view!.saved).toBe(false);
    store().setDesk([1.2, 0.6]);
    expect(await store().save()).toBe(true);
    expect(store().view!.saved).toBe(true);
    expect(Object.keys(store().saved.models).sort()).toEqual(models.map(m => m.asset).sort());
    expect(store().history.past).toHaveLength(1);
    expect(store().draft).toEqual(store().saved);
  });

  it('refuses to save a stop without a name', async () => {
    await openMockWorkspace();
    await store().load('frostveil');
    store().setStop('arrival', { name: ' ' });
    expect(await store().save()).toBe(false);
    expect(store().view!.layout.stops[0]!.name).toBe('Arrival');
  });

  it('flags an agent change instead of dropping unsaved edits', async () => {
    await openMockWorkspace();
    await store().load('frostveil');
    store().setDesk([1, 1]);
    await store().onChanged('frostveil');
    expect(store().outside).toBe(true);
    expect(store().draft.desk).toEqual([1, 1]);
    store().revert();
    expect(store().draft.desk).toBeNull();
    expect(store().outside).toBe(false);
  });

  it('reloads an agent change when nothing is unsaved', async () => {
    await openMockWorkspace();
    await store().load('frostveil');
    const layout = { ...store().view!.layout, desk: { size: [2, 1] as [number, number] } };
    await commands.layoutSave('frostveil', layout);
    await store().onChanged('frostveil');
    expect(store().draft.desk).toEqual([2, 1]);
  });
});
