import { afterEach, describe, expect, it } from 'vitest';
import { commands } from '@/ipc/client';
import { clearMocks, openPlans } from '@/test/open-plan';
import { ANSWER_FIRST } from './plans-sync';
import { useAppStore } from './use-app-store';
import { usePlanExtrasStore } from './use-plan-extras-store';
import { usePlanLogStore } from './use-plan-log-store';
import { usePlansStore } from './use-plans-store';

afterEach(clearMocks);

const COMBAT = { project: 'frostveil', id: 'combat-ideas' };

const asked = async () => {
  const backend = await openPlans();
  await usePlansStore.getState().openCanvas('combat-ideas');
  await backend.mock('mock_plan_propose', { ...COMBAT, nodes: ['n-06'], reason: 'Answered.' });
  await usePlanExtrasStore.getState().load(COMBAT.project, COMBAT.id);
  const [proposal] = usePlanExtrasStore.getState().proposals;
  return proposal!;
};

describe('the canvas extras store', () => {
  it('removes what an agent asked as the person, saving their edits first', async () => {
    const proposal = await asked();
    usePlansStore.getState().updateNode('n-02', { text: 'Unsaved rune idea' });
    expect(await usePlanExtrasStore.getState().answer(proposal.id, true)).toBe(true);
    const open = usePlansStore.getState().open;
    expect(open?.plan.nodes.some(n => n.id === 'n-06')).toBe(false);
    expect(open?.plan.nodes.find(n => n.id === 'n-02')?.text).toBe('Unsaved rune idea');
    expect(usePlanExtrasStore.getState().proposals).toEqual([]);
    expect(usePlanLogStore.getState().lines[0]).toMatchObject({ by: null, undoable: true });
  });

  it('keeps it, and says so when the core refuses', async () => {
    const proposal = await asked();
    expect(await usePlanExtrasStore.getState().answer(proposal.id, false)).toBe(true);
    expect(usePlansStore.getState().open?.plan.nodes.some(n => n.id === 'n-06')).toBe(true);
    expect(await usePlanExtrasStore.getState().answer(proposal.id, true)).toBe(false);
    expect(useAppStore.getState().notice?.title).toBe('The nodes were not removed');
  });

  it('refuses Remove when its links changed since the panel showed them, and shows them again', async () => {
    const proposal = await asked();
    // A link was drawn after the panel read the proposal: it still says 0.
    const shown = { ...proposal, links: 1 };
    usePlanExtrasStore.setState({ proposals: [shown] });
    expect(await usePlanExtrasStore.getState().answer(proposal.id, true)).toBe(false);
    expect(useAppStore.getState().notice?.detail).toContain('not the 1 shown');
    expect(usePlansStore.getState().open?.plan.nodes.some(n => n.id === 'n-06')).toBe(true);
    expect(usePlanExtrasStore.getState().proposals[0]?.links).toBe(0);
    expect(await usePlanExtrasStore.getState().answer(proposal.id, true)).toBe(true);
  });

  it('waits for an answer to an outside change before writing', async () => {
    const proposal = await asked();
    const doc = await commands.planGet(COMBAT.project, COMBAT.id);
    usePlansStore.setState({ outside: { doc, conflicts: ['n-01'] } });
    expect(await usePlanExtrasStore.getState().answer(proposal.id, true)).toBe(false);
    expect(await usePlanExtrasStore.getState().startFrom('n-01')).toBeNull();
    expect(useAppStore.getState().notice?.title).toBe(ANSWER_FIRST);
    expect(usePlanExtrasStore.getState().proposals).toHaveLength(1);
  });
});
