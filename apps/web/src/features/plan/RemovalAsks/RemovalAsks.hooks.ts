import type { AskedState, RemovalProposal } from '@/ipc/client';
import { usePlanExtrasStore } from '@/state/use-plan-extras-store';

const STATE_NOTE: Record<AskedState, string> = {
  here: '',
  changed: 'changed since',
  gone: 'gone',
};

/** How a proposal reads in the panel: who asks, about what, and why. */
export function headOf(p: RemovalProposal) {
  const n = p.nodes.length;
  return `${p.by} wants to remove ${n === 1 ? 'a node' : `${n} nodes`}`;
}

/**
 * The removal proposals waiting for the person, oldest first, each with
 * Keep it and Remove. A stale one (a node changed or went since the agent
 * asked) can only be kept: nothing is removed that the agent did not see.
 */
export function useRemovalAsks() {
  const proposals = usePlanExtrasStore(s => s.proposals);
  const busy = usePlanExtrasStore(s => s.busy);
  const answer = usePlanExtrasStore(s => s.answer);
  return {
    proposals,
    busy,
    noteOf: (state: AskedState) => STATE_NOTE[state],
    namesOf: (p: RemovalProposal) => p.nodes.map(n => n.name).join(', '),
    keep: (id: string) => void answer(id, false),
    remove: (id: string) => void answer(id, true),
  };
}
