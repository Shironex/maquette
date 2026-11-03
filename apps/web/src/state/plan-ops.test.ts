import { describe, expect, it } from 'vitest';
import type { PlanFile } from '@/ipc/client';
import { demoCombat, note } from '@/ipc/mock/plan-fixtures';
import { EMPTY_HISTORY, HISTORY_MAX, record, takeRedo, takeUndo } from './plan-history';
import { applyOps, conflictsOf, diffPlans, invertOps, rebase } from './plan-ops';

const withNode = (plan: PlanFile, id: string, fields: Partial<PlanFile['nodes'][number]>) => ({
  ...plan,
  nodes: plan.nodes.map(n => (n.id === id ? { ...n, ...fields } : n)),
});

describe('plan operations', () => {
  it('turns one canvas into another and back', () => {
    const base = demoCombat();
    const next = {
      ...withNode(base, 'n-01', { x: 140, text: 'Moved and edited' }),
      nodes: [
        ...withNode(base, 'n-01', { x: 140, text: 'Moved and edited' }).nodes.filter(
          n => n.id !== 'n-06'
        ),
        note('n-09', 0, 600, 'New'),
      ],
      edges: base.edges.filter(e => e.id !== 'e-01'),
    };
    const ops = diffPlans(base, next);
    expect(ops.map(o => o.op)).toEqual(['remove-edge', 'remove-node', 'set-node', 'add-node']);
    expect(applyOps(base, ops, 'mine').plan.nodes).toEqual(next.nodes);
    const back = applyOps(next, invertOps(ops), 'careful');
    expect(back.skipped).toBe(0);
    expect(new Set(back.plan.nodes.map(n => n.id))).toEqual(new Set(base.nodes.map(n => n.id)));
    expect(back.plan.edges.map(e => e.id).sort()).toEqual(['e-01', 'e-02', 'e-03']);
  });

  it('keeps a node someone linked since, and their link, when undoing carefully', () => {
    const base = demoCombat();
    const added = { ...base, nodes: [...base.nodes, note('n-09', 0, 600, 'Agent note')] };
    const add = diffPlans(base, added);
    const link = { ...base.edges[0]!, id: 'e-person', from: 'n-01', to: 'n-09' };
    const linked = { ...added, edges: [...added.edges, link] };
    const undone = applyOps(linked, invertOps(add), 'careful');
    expect(undone.skipped).toBe(1);
    expect(undone.plan.nodes.map(n => n.id)).toContain('n-09');
    expect(undone.plan.edges.map(e => e.id)).toContain('e-person');
    // Written as the person's own removal it goes, links and all.
    expect(applyOps(linked, invertOps(add), 'mine').plan.edges.map(e => e.id)).not.toContain(
      'e-person'
    );
  });

  it('leaves a field changed since alone when applied carefully', () => {
    const base = demoCombat();
    const mine = withNode(base, 'n-01', { text: 'Mine' });
    const undo = invertOps(diffPlans(base, mine));
    const agentSince = withNode(mine, 'n-01', { text: 'The agent rewrote it' });
    const careful = applyOps(agentSince, undo, 'careful');
    expect(careful.skipped).toBe(1);
    expect(careful.plan.nodes.find(n => n.id === 'n-01')?.text).toBe('The agent rewrote it');
    // A field nobody else touched still goes back.
    const moved = withNode(base, 'n-02', { x: 500 });
    const back = applyOps(
      withNode(moved, 'n-01', { text: 'Other' }),
      invertOps(diffPlans(base, moved)),
      'careful'
    );
    expect(back.plan.nodes.find(n => n.id === 'n-02')?.x).toBe(60);
  });

  it('merges edits to different nodes and finds clashes on the same field', () => {
    const base = demoCombat();
    const mine = withNode(base, 'n-01', { text: 'Mine' });
    const theirs = withNode(base, 'n-02', { text: 'Theirs' });
    expect(conflictsOf(base, mine, theirs)).toEqual([]);
    const merged = rebase(base, mine, theirs, 'mine');
    expect(merged.nodes.find(n => n.id === 'n-01')?.text).toBe('Mine');
    expect(merged.nodes.find(n => n.id === 'n-02')?.text).toBe('Theirs');

    const clash = withNode(base, 'n-01', { text: 'Theirs too' });
    expect(conflictsOf(base, mine, clash)).toEqual(['n-01']);
    expect(rebase(base, mine, clash, 'mine').nodes.find(n => n.id === 'n-01')?.text).toBe('Mine');
    expect(rebase(base, mine, clash, 'theirs').nodes.find(n => n.id === 'n-01')?.text).toBe(
      'Theirs too'
    );
    // The same change on both sides is no clash.
    expect(conflictsOf(base, mine, mine)).toEqual([]);
  });

  it('counts an edit to a node the other side removed as a clash', () => {
    const base = demoCombat();
    const mine = withNode(base, 'n-06', { text: 'Edited' });
    const theirs = { ...base, nodes: base.nodes.filter(n => n.id !== 'n-06') };
    expect(conflictsOf(base, mine, theirs)).toEqual(['n-06']);
    expect(rebase(base, mine, theirs, 'theirs').nodes.map(n => n.id)).not.toContain('n-06');
    expect(rebase(base, mine, theirs, 'mine').nodes.find(n => n.id === 'n-06')?.text).toBe(
      'Edited'
    );
  });
});

describe('plan history', () => {
  it('undoes and redoes edits in order and drops redo after a new edit', () => {
    const base = demoCombat();
    const one = diffPlans(base, withNode(base, 'n-01', { x: 1 }));
    const two = diffPlans(base, withNode(base, 'n-02', { x: 2 }));
    let h = record(record(EMPTY_HISTORY, one), two);
    expect(record(h, [])).toBe(h);
    const undo = takeUndo(h);
    expect(undo?.ops).toEqual(invertOps(two));
    h = undo!.history;
    expect(takeRedo(h)?.ops).toEqual(two);
    h = record(h, one);
    expect(h.future).toEqual([]);
    expect(takeRedo(EMPTY_HISTORY)).toBeNull();
    expect(takeUndo(EMPTY_HISTORY)).toBeNull();
  });

  it('keeps the newest edits only', () => {
    const base = demoCombat();
    let h = EMPTY_HISTORY;
    for (let i = 0; i < HISTORY_MAX + 5; i += 1)
      h = record(h, diffPlans(base, withNode(base, 'n-01', { x: i + 1 })));
    expect(h.past).toHaveLength(HISTORY_MAX);
  });

  it('turns the canvas settings around as the core does', () => {
    const base = demoCombat();
    const mark = { a: 'n-01', b: 'n-02', key: '0a1b2c3d' };
    const ops = [
      { op: 'set-auto-layout', before: true, after: false },
      { op: 'mark-fine', mark },
    ] as const;
    const done = applyOps(base, ops, 'mine');
    expect(done.plan.autoLayout).toBe(false);
    expect(done.plan.fine).toEqual([mark]);
    const back = applyOps(done.plan, invertOps(ops), 'careful');
    expect(back.skipped).toBe(0);
    expect(back.plan).toEqual(base);
    // A mark made again for new words stays through a careful undo.
    const again = { ...done.plan, fine: [{ ...mark, key: 'ffff0000' }] };
    expect(applyOps(again, invertOps(ops), 'careful').plan.fine).toHaveLength(1);
  });
});
