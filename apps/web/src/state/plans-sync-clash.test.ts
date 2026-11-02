import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { clearMocks, openPlans } from '@/test/open-plan';
import { ANSWER_FIRST, RELOAD_RETRY_MS } from './plans-sync';
import { useAppStore } from './use-app-store';
import { NOTHING_TO_UNDO, usePlanLogStore } from './use-plan-log-store';
import { SAVE_DELAY, usePlansStore } from './use-plans-store';

beforeEach(() => vi.useFakeTimers({ shouldAdvanceTime: true }));
afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  clearMocks();
});

const plans = () => usePlansStore.getState();
const ORIGINAL = 'Slow time around you for a moment. Hold Shift, the same key as Dash.';
const flowNode = (id: string) => plans().nodes.find(n => n.id === id);
const fileText = async (id: string) =>
  (await commands.planGet('frostveil', 'combat-ideas')).plan.nodes.find(n => n.id === id)?.text;
const outside = (store: Awaited<ReturnType<typeof openPlans>>, node: string, text: string) =>
  store.mock('mock_plan_outside', { project: 'frostveil', id: 'combat-ideas', node, text });

/** The person and a sync both changed n-01: the dialog asks which wins. */
async function clash() {
  const store = await openPlans();
  await plans().openCanvas('combat-ideas');
  plans().updateNode('n-01', { text: 'Mine' });
  await outside(store, 'n-01', 'Theirs');
  await plans().reload();
  expect(plans().outside?.conflicts).toEqual(['n-01']);
  return { store, apply: vi.spyOn(commands, 'planApply') };
}

/** Nothing chose for the person: the question is still up and the file holds theirs. */
async function stillAsks(apply: ReturnType<typeof vi.spyOn>) {
  expect(apply).not.toHaveBeenCalled();
  expect(plans().outside).not.toBeNull();
  expect(plans().open?.id).toBe('combat-ideas');
  expect(flowNode('n-01')?.data.node.text).toBe('Mine');
  expect(await fileText('n-01')).toBe('Theirs');
}

describe('while the outside change waits for its answer', () => {
  it('a save says to answer first instead of keeping mine', async () => {
    const { apply } = await clash();
    expect(await plans().save()).toBe(false);
    expect(useAppStore.getState().notice?.title).toBe(ANSWER_FIRST);
    await vi.advanceTimersByTimeAsync(SAVE_DELAY * 4);
    await stillAsks(apply);
  });

  it('leaving for the list, another canvas or another project waits', async () => {
    const { apply } = await clash();
    expect(await plans().close()).toBe(false);
    await plans().openCanvas('boss-frost-warden');
    await plans().loadList('moon-island');
    expect(plans().leaving).toBeNull();
    expect(plans().project).toBe('frostveil');
    await stillAsks(apply);
  });

  it('quitting answers why it cannot save yet', async () => {
    const { apply } = await clash();
    expect(await plans().saveForQuit()).toMatch(/Keep mine or Use theirs/);
    await stillAsks(apply);
  });

  it('renaming waits', async () => {
    const { apply } = await clash();
    const rename = vi.spyOn(commands, 'planRename');
    expect(await plans().rename('combat-ideas', 'Combat')).toBe(false);
    expect(rename).not.toHaveBeenCalled();
    await stillAsks(apply);
  });

  it('a link still opens, without saving', async () => {
    const { apply } = await clash();
    const link = vi.spyOn(commands, 'planOpenLink').mockResolvedValue(null);
    await plans().openLink('n-05');
    expect(link).toHaveBeenCalled();
    await stillAsks(apply);
  });

  it('an undo from the log waits', async () => {
    const { apply } = await clash();
    const undo = vi.spyOn(commands, 'planUndo');
    await usePlanLogStore.getState().load('frostveil', 'combat-ideas');
    const line = usePlanLogStore.getState().lines[0];
    expect(await usePlanLogStore.getState().undo([line!.id])).toBe(false);
    expect(undo).not.toHaveBeenCalled();
    await stillAsks(apply);
  });

  it('drops the question when the file goes back to what the view had', async () => {
    const { store } = await clash();
    await outside(store, 'n-01', ORIGINAL);
    await plans().reload();
    expect(plans().outside).toBeNull();
    await vi.advanceTimersByTimeAsync(SAVE_DELAY + 50);
    expect(await fileText('n-01')).toBe('Mine');
  });

  it('saves once the person answers', async () => {
    await clash();
    plans().keepMine();
    await vi.advanceTimersByTimeAsync(SAVE_DELAY + 50);
    expect(await fileText('n-01')).toBe('Mine');
  });
});

describe('reading the file again', () => {
  it('reads again when a save ran while it read, so it never shows a stale file', async () => {
    const store = await openPlans();
    await plans().openCanvas('combat-ideas');
    await outside(store, 'n-02', 'From the laptop');
    const real = commands.planGet;
    const get = vi.spyOn(commands, 'planGet').mockImplementationOnce(async (project, id) => {
      const doc = await real(project, id);
      // The person edits and saves while the first read is on its way.
      plans().updateNode('n-01', { text: 'Saved meanwhile' });
      void plans().save();
      return doc;
    });
    await plans().reload();
    expect(get).toHaveBeenCalledTimes(2);
    expect(plans().outside).toBeNull();
    expect(flowNode('n-01')?.data.node.text).toBe('Saved meanwhile');
    expect(flowNode('n-02')?.data.node.text).toBe('From the laptop');
    expect(await fileText('n-01')).toBe('Saved meanwhile');
  });

  it('reads again a moment later when saves keep overtaking it', async () => {
    await openPlans();
    await plans().openCanvas('combat-ideas');
    const real = commands.planGet;
    let n = 0;
    const get = vi.spyOn(commands, 'planGet').mockImplementation(async (project, id) => {
      const doc = await real(project, id);
      n += 1;
      if (n <= 5) {
        plans().updateNode('n-01', { text: `Saved ${n}` });
        void plans().save();
      }
      return doc;
    });
    await plans().reload();
    expect(get).toHaveBeenCalledTimes(5);
    await vi.advanceTimersByTimeAsync(RELOAD_RETRY_MS + 50);
    expect(get).toHaveBeenCalledTimes(6);
    expect(flowNode('n-01')?.data.node.text).toBe('Saved 5');
  });

  it('keeps where a drag started when an outside change lands mid drag', async () => {
    const store = await openPlans();
    await plans().openCanvas('combat-ideas');
    const startX = flowNode('n-01')!.position.x;
    plans().onNodesChange([
      { type: 'position', id: 'n-01', position: { x: 300, y: 80 }, dragging: true },
    ]);
    await outside(store, 'n-02', 'From the laptop');
    await plans().reload();
    plans().onNodesChange([
      { type: 'position', id: 'n-01', position: { x: 340, y: 80 }, dragging: false },
    ]);
    expect(plans().history.past).toHaveLength(1);
    plans().undo();
    expect(flowNode('n-01')?.position.x).toBe(startX);
    expect(flowNode('n-02')?.data.node.text).toBe('From the laptop');
  });
});

describe('freeing what the log keeps for undo', () => {
  it('says plainly when an undo finds nothing left to undo, and marks it undone', async () => {
    await openPlans();
    await plans().openCanvas('combat-ideas');
    // The person removes what the agent added a day ago.
    plans().onNodesChange([{ type: 'remove', id: 'n-05' }]);
    await plans().save();
    const log = usePlanLogStore.getState();
    await log.load('frostveil', 'combat-ideas');
    const agent = usePlanLogStore.getState().lines.find(l => l.by === 'Research agent');
    expect(await usePlanLogStore.getState().undo([agent!.id])).toBe(true);
    expect(useAppStore.getState().notice?.title).toBe(NOTHING_TO_UNDO);
    const lines = usePlanLogStore.getState().lines;
    expect(lines.find(l => l.id === agent!.id)?.undone).toBe(true);
    expect(lines[0]?.summary).toMatch(/^Nothing to undo, already changed/);
  });

  it('asks before a Keep takes an earlier change along, and keeps only what was confirmed', async () => {
    const store = await openPlans();
    await plans().openCanvas('combat-ideas');
    await store.mock('mock_plan_agent', { project: 'frostveil', id: 'combat-ideas', edit: 'n-02' });
    await store.mock('mock_plan_agent', { project: 'frostveil', id: 'combat-ideas', edit: 'n-02' });
    const log = usePlanLogStore.getState();
    await log.load('frostveil', 'combat-ideas');
    const [newer, older] = usePlanLogStore.getState().lines;
    expect(await usePlanLogStore.getState().keep([newer!.id])).toBe(false);
    expect(usePlanLogStore.getState().keepAsk?.needs.map(n => n.id)).toEqual([older!.id]);
    // The core refuses a short list too.
    await expect(
      commands.planKeep('frostveil', 'combat-ideas', [newer!.id], [])
    ).rejects.toBeTruthy();
    // An id the list never named is refused as well.
    await expect(
      commands.planKeep('frostveil', 'combat-ideas', [newer!.id], [older!.id, 'c-elsewhere'])
    ).rejects.toMatchObject({ message: expect.stringMatching(/Review again/) as unknown });
    usePlanLogStore.getState().cancelKeep();
    expect(usePlanLogStore.getState().lines.some(l => l.kept)).toBe(false);
    await usePlanLogStore.getState().keep([newer!.id]);
    expect(await usePlanLogStore.getState().keepAll()).toBe(true);
    const kept = usePlanLogStore
      .getState()
      .lines.filter(l => l.kept)
      .map(l => l.id);
    expect(kept.sort()).toEqual([newer!.id, older!.id].sort());
  });

  it('refuses to undo an undo and offers no Undo on it', async () => {
    await openPlans();
    await plans().openCanvas('combat-ideas');
    plans().onNodesChange([{ type: 'remove', id: 'n-05' }]);
    await plans().save();
    await usePlanLogStore.getState().load('frostveil', 'combat-ideas');
    const agent = usePlanLogStore.getState().lines.find(l => l.by === 'Research agent');
    await usePlanLogStore.getState().undo([agent!.id]);
    const undoLine = usePlanLogStore.getState().lines[0];
    expect(undoLine?.mark).toBe('undo');
    expect(undoLine?.undoable).toBe(false);
    expect(await usePlanLogStore.getState().undo([undoLine!.id])).toBe(false);
    expect(useAppStore.getState().notice?.detail).toMatch(/an undo can not be undone/);
  });

  it('keeps an agent edit of the person note, which can then no longer be kept', async () => {
    const store = await openPlans();
    await plans().openCanvas('combat-ideas');
    await store.mock('mock_plan_agent', { project: 'frostveil', id: 'combat-ideas' });
    await usePlanLogStore.getState().load('frostveil', 'combat-ideas');
    const edit = usePlanLogStore.getState().lines[0];
    expect(edit?.keepable).toBe(true);
    expect(await usePlanLogStore.getState().keep([edit!.id])).toBe(true);
    const after = usePlanLogStore.getState().lines;
    expect(after[0]?.mark).toBe('keep');
    expect(after.find(l => l.id === edit!.id)).toMatchObject({ kept: true, keepable: false });
  });
});
