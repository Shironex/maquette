import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { clearMocks, openPlans } from '@/test/open-plan';
import { usePlanLogStore } from './use-plan-log-store';
import { SAVE_DELAY, usePlansStore } from './use-plans-store';

beforeEach(() => vi.useFakeTimers({ shouldAdvanceTime: true }));
afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  clearMocks();
});

const plans = () => usePlansStore.getState();
const flowText = (id: string) => plans().nodes.find(n => n.id === id)?.data.node.text;
const fileText = async (id: string) =>
  (await commands.planGet('frostveil', 'combat-ideas')).plan.nodes.find(n => n.id === id)?.text;

async function openCombat() {
  const store = await openPlans();
  await plans().openCanvas('combat-ideas');
  return store;
}

describe('undo and redo of the person edits', () => {
  it('undoes a move and an edit in turn, redoes them, and saves each', async () => {
    await openCombat();
    plans().onNodesChange([
      { type: 'position', id: 'n-01', position: { x: 200, y: 80 }, dragging: false },
    ]);
    plans().updateNode('n-02', { text: 'Edited' });
    expect(plans().history.past).toHaveLength(2);

    plans().undo();
    expect(flowText('n-02')).toBe('Every third hit leaves a rune. Three runes in a row burst.');
    plans().undo();
    expect(plans().nodes.find(n => n.id === 'n-01')?.position.x).toBe(60);
    expect(plans().history.future).toHaveLength(2);
    await vi.advanceTimersByTimeAsync(SAVE_DELAY + 50);
    expect(plans().dirty).toBe(false);

    plans().redo();
    plans().redo();
    expect(flowText('n-02')).toBe('Edited');
    await plans().save();
    expect(await fileText('n-02')).toBe('Edited');
  });

  it('records a drag once, when it ends', async () => {
    await openCombat();
    for (const x of [70, 80, 90])
      plans().onNodesChange([
        { type: 'position', id: 'n-01', position: { x, y: 80 }, dragging: true },
      ]);
    plans().onNodesChange([
      { type: 'position', id: 'n-01', position: { x: 100, y: 80 }, dragging: false },
    ]);
    expect(plans().history.past).toHaveLength(1);
  });

  it('never undoes an agent write with the person edit', async () => {
    const store = await openCombat();
    plans().updateNode('n-01', { text: 'Mine' });
    await plans().save();
    await store.mock('mock_plan_agent', { project: 'frostveil', id: 'combat-ideas' });
    await plans().reload();
    expect(plans().nodes.map(n => n.id)).toContain('n-a1');
    plans().undo();
    await plans().save();
    expect(flowText('n-01')).toBe(
      'Slow time around you for a moment. Hold Shift, the same key as Dash.'
    );
    expect(plans().nodes.map(n => n.id)).toContain('n-a1');
  });
});

describe('writes from outside', () => {
  it('keeps an agent write the person save would otherwise write over', async () => {
    const store = await openCombat();
    await store.mock('mock_plan_agent', { project: 'frostveil', id: 'combat-ideas' });
    // The person edits before the view saw the agent write, then saves.
    plans().updateNode('n-01', { text: 'Mine, made before I saw it' });
    await plans().save();
    const file = await commands.planGet('frostveil', 'combat-ideas');
    expect(file.plan.nodes.map(n => n.id)).toEqual(
      expect.arrayContaining(['n-a1', 'n-a2', 'n-a3'])
    );
    expect(file.plan.nodes.find(n => n.id === 'n-01')?.text).toBe('Mine, made before I saw it');
    // The save's answer shows the agent's nodes, marked as added.
    expect(plans().nodes.map(n => n.id)).toContain('n-a1');
    expect(plans().marks['n-a1']).toBe('added');
    expect(plans().marks['n-06']).toBe('edited');
  });

  it('merges an outside write with unsaved edits to other nodes without asking', async () => {
    const store = await openCombat();
    plans().updateNode('n-01', { text: 'Unsaved' });
    await store.mock('mock_plan_outside', {
      project: 'frostveil',
      id: 'combat-ideas',
      node: 'n-02',
      text: 'From the laptop',
    });
    await plans().reload();
    expect(plans().outside).toBeNull();
    expect(flowText('n-01')).toBe('Unsaved');
    expect(flowText('n-02')).toBe('From the laptop');
    await vi.advanceTimersByTimeAsync(SAVE_DELAY + 50);
    expect(await fileText('n-01')).toBe('Unsaved');
    expect(await fileText('n-02')).toBe('From the laptop');
  });

  it('asks when both changed the same node, and holds autosave until answered', async () => {
    const store = await openCombat();
    const apply = vi.spyOn(commands, 'planApply');
    plans().updateNode('n-01', { text: 'Mine' });
    await store.mock('mock_plan_outside', {
      project: 'frostveil',
      id: 'combat-ideas',
      node: 'n-01',
      text: 'Theirs',
    });
    await plans().reload();
    expect(plans().outside?.conflicts).toEqual(['n-01']);
    await vi.advanceTimersByTimeAsync(SAVE_DELAY * 4);
    expect(apply).not.toHaveBeenCalled();
    expect(flowText('n-01')).toBe('Mine');

    plans().useTheirs();
    expect(plans().outside).toBeNull();
    expect(flowText('n-01')).toBe('Theirs');
    expect(plans().dirty).toBe(false);
  });

  it('keeps mine when asked, and saves it', async () => {
    const store = await openCombat();
    plans().updateNode('n-01', { text: 'Mine' });
    await store.mock('mock_plan_outside', {
      project: 'frostveil',
      id: 'combat-ideas',
      node: 'n-01',
      text: 'Theirs',
    });
    await plans().reload();
    plans().keepMine();
    await vi.advanceTimersByTimeAsync(SAVE_DELAY + 50);
    expect(await fileText('n-01')).toBe('Mine');
  });

  it('reads its own save back without reloading in a loop', async () => {
    await openCombat();
    plans().updateNode('n-01', { text: 'Saved once' });
    await plans().save();
    const get = vi.spyOn(commands, 'planGet');
    const apply = vi.spyOn(commands, 'planApply');
    const before = plans().open;
    await plans().reload();
    await vi.advanceTimersByTimeAsync(SAVE_DELAY * 4);
    expect(get).toHaveBeenCalledTimes(1);
    expect(apply).not.toHaveBeenCalled();
    expect(plans().open).toBe(before);
  });
});

describe('the change log', () => {
  it('lists the person save and an agent run, and undoes the run from it', async () => {
    const store = await openCombat();
    const log = usePlanLogStore.getState();
    await log.load('frostveil', 'combat-ideas');
    expect(usePlanLogStore.getState().lines[0]?.summary).toBe('Moved 1 note');

    // Later than the canvas opened, so it counts as new.
    await vi.advanceTimersByTimeAsync(1000);
    await store.mock('mock_plan_agent', { project: 'frostveil', id: 'combat-ideas' });
    await usePlanLogStore.getState().load('frostveil', 'combat-ideas');
    const toast = usePlanLogStore.getState().toast;
    expect(toast?.by).toBe('Research agent');

    plans().updateNode('n-02', { text: 'Unsaved, kept through the undo' });
    expect(await usePlanLogStore.getState().undo([toast!.id])).toBe(true);
    expect(plans().nodes.map(n => n.id)).not.toContain('n-a1');
    expect(flowText('n-02')).toBe('Unsaved, kept through the undo');
    const lines = usePlanLogStore.getState().lines;
    expect(lines[0]?.mark).toBe('undo');
    expect(lines.find(l => l.id === toast!.id)?.undone).toBe(true);
    expect(usePlanLogStore.getState().toast).toBeNull();
  });
});
