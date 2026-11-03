import { describe, expect, it } from 'vitest';
import type { PlanDoc, PlanFile, PlanLogLine, RemovalProposal } from '../client';
import { createBackend } from './backend';

/** The mock backend with Frostveil's demo canvases and Combat systems. */
function setup() {
  const handle = createBackend({
    scenario: 'configured',
    probeDelay: 0,
    pace: 0,
    conceptMs: 60_000,
    plans: 'systems',
  });
  const run = <T>(cmd: string, args: Record<string, unknown> = {}) =>
    handle(cmd, { project: 'frostveil', ...args }) as T;
  return { run };
}

const COMBAT = { id: 'combat-ideas' };
/** The opened canvas's `created`, which every answer and setting names. */
const createdOf = async (run: ReturnType<typeof setup>['run'], id: string) =>
  (await run<PlanDoc>('plan_get', { id })).plan.created;

describe('mock canvas extras', () => {
  it('lists an agent proposal, refuses it stale and removes it as the person', async () => {
    const { run } = setup();
    await run('mock_plan_propose', { ...COMBAT, nodes: ['n-06'], reason: 'Answered elsewhere.' });
    const [open] = await run<RemovalProposal[]>('plan_proposals', COMBAT);
    expect(open).toMatchObject({
      by: 'Research agent',
      reason: 'Answered elsewhere.',
      stale: false,
      links: 0,
    });
    const created = await createdOf(run, COMBAT.id);
    await expect(
      run('plan_answer_removal', { ...COMBAT, created: 'other', proposal: open?.id, remove: true })
    ).rejects.toMatchObject({ code: 'invalid' });
    expect(open?.nodes).toEqual([
      { id: 'n-06', name: 'Does the veil read on a phone screen?', state: 'here' },
    ]);
    const done = await run<PlanDoc>('plan_answer_removal', {
      ...COMBAT,
      created,
      proposal: open?.id,
      remove: true,
    });
    expect(done.plan.nodes.some(n => n.id === 'n-06')).toBe(false);
    expect(await run<RemovalProposal[]>('plan_proposals', COMBAT)).toEqual([]);
    const [line] = await run<PlanLogLine[]>('plan_log', { ...COMBAT, limit: 5 });
    expect(line).toMatchObject({
      by: null,
      undoable: true,
      summary: 'Removed Does the veil read on a phone screen?, as Research agent asked',
    });

    // A proposal for a node that changed is stale: Remove is refused, Keep it clears it.
    await run('mock_plan_propose', { ...COMBAT, nodes: ['n-02'] });
    await run('mock_plan_outside', { ...COMBAT, node: 'n-02', text: 'Two runes now.' });
    const [stale] = await run<RemovalProposal[]>('plan_proposals', COMBAT);
    expect(stale?.stale).toBe(true);
    expect(stale?.nodes[0]?.state).toBe('changed');
    await expect(
      run('plan_answer_removal', { ...COMBAT, created, proposal: stale?.id, remove: true })
    ).rejects.toMatchObject({ code: 'invalid' });
    await run('plan_answer_removal', { ...COMBAT, created, proposal: stale?.id, remove: false });
    expect(await run<RemovalProposal[]>('plan_proposals', COMBAT)).toEqual([]);
  });

  it('starts a canvas from a node and its links, leaving the source as it was', async () => {
    const { run } = setup();
    const before = (await run<PlanDoc>('plan_get', COMBAT)).plan;
    const made = await run<PlanDoc>('plan_start_from', {
      source: 'combat-ideas',
      node: 'n-03',
      name: null,
    });
    expect(made.id).toBe('veil-gate-at-night');
    expect(made.plan.nodes).toHaveLength(3);
    expect(made.plan.edges).toHaveLength(2);
    const seed = made.plan.nodes.find(n => n.title === 'Veil gate at night');
    expect([seed?.x, seed?.y]).toEqual([0, 0]);
    expect((await run<PlanDoc>('plan_get', COMBAT)).plan).toEqual(before);
    const [line] = await run<PlanLogLine[]>('plan_log', { id: made.id, limit: 5 });
    expect(line?.summary).toBe('Started from Veil gate at night on Combat ideas');
  });

  it('switches auto layout and marks an overlap fine, both undoable', async () => {
    const { run } = setup();
    const created = await createdOf(run, COMBAT.id);
    await expect(
      run('plan_set_auto_layout', { ...COMBAT, created: 'other', on: false })
    ).rejects.toMatchObject({ code: 'invalid' });
    const off = await run<PlanDoc>('plan_set_auto_layout', { ...COMBAT, created, on: false });
    expect(off.plan.autoLayout).toBe(false);
    const [line] = await run<PlanLogLine[]>('plan_log', { ...COMBAT, limit: 5 });
    await run('plan_undo', { ...COMBAT, changes: [line?.id] });
    expect((await run<PlanDoc>('plan_get', COMBAT)).plan.autoLayout ?? true).toBe(true);

    const systems = { id: 'combat-systems' };
    const shown = await run<PlanDoc>('plan_get', systems);
    const flagged = shown.overlaps?.find(o => !o.fine);
    expect(flagged).toBeDefined();
    const marked = await run<PlanDoc>('plan_mark_fine', {
      ...systems,
      created: shown.plan.created,
      a: flagged?.b,
      b: flagged?.a,
    });
    expect(marked.overlaps?.find(o => o.a === flagged?.a && o.b === flagged?.b)?.fine).toBe(true);
    await expect(
      run('plan_mark_fine', {
        ...systems,
        created: shown.plan.created,
        a: flagged?.a,
        b: flagged?.b,
      })
    ).rejects.toMatchObject({ code: 'invalid' });
    const plan: PlanFile = marked.plan;
    expect(plan.version).toBe(4);
  });
});
