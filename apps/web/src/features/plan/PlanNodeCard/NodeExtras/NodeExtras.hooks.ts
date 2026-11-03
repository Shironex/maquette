import type { OverlapPart, PlanNode } from '@/ipc/client';
import { usePlanExtrasStore } from '@/state/use-plan-extras-store';
import { usePlansStore } from '@/state/use-plans-store';

const PART: Record<OverlapPart, string> = { verb: 'verb', look: 'look', input: 'key' };

/** A node's name for a flag: its title, else its first line, else its id. */
export const shortName = (node: PlanNode | undefined, id: string) =>
  node?.title.trim() || node?.text.trim().split('\n')[0]?.trim() || id;

/** Whether an agent's open removal proposal names this node. */
export function useAsked(id: string) {
  return usePlanExtrasStore(s =>
    s.proposals.some(p => p.nodes.some(n => n.id === id && n.state !== 'gone'))
  );
}

/** "Same key as Frost veil (shift)" for each overlap of this card not marked as fine. */
export function useOverlapFlags(id: string) {
  const overlaps = usePlansStore(s => s.open?.overlaps);
  const plan = usePlansStore(s => s.open?.plan);
  return (overlaps ?? [])
    .filter(o => !o.fine && (o.a === id || o.b === id))
    .map(o => {
      const other = o.a === id ? o.b : o.a;
      const name = shortName(
        plan?.nodes.find(n => n.id === other),
        other
      );
      return {
        key: `${o.a}/${o.b}/${o.part}`,
        text: `Same ${PART[o.part]} as ${name} (${o.word})`,
      };
    });
}

/** Start a canvas from this node: not from a camera stop, which lives only in the camera plan. */
export function useNodeTools(node: PlanNode) {
  const startFrom = usePlanExtrasStore(s => s.startFrom);
  return {
    canStart: node.kind !== 'stop',
    start: () => void startFrom(node.id),
  };
}
