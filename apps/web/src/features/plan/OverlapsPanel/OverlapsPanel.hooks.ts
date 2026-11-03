import type { Overlap } from '@/ipc/client';
import { usePlanExtrasStore } from '@/state/use-plan-extras-store';
import { usePlansStore } from '@/state/use-plans-store';
import { shortName } from '../PlanNodeCard/NodeExtras/NodeExtras.hooks';

/** One overlap as the panel words it. */
export interface OverlapView {
  key: string;
  a: string;
  b: string;
  title: string;
  detail: string;
  fine: boolean;
}

const cap = (word: string) => word.charAt(0).toUpperCase() + word.slice(1);

function wordsOf(o: Overlap, name: (id: string) => string) {
  const [a, b] = [name(o.a), name(o.b)];
  if (o.part === 'input')
    return {
      title: `${a} and ${b} share ${cap(o.word)}`,
      detail: `Both use ${o.word} as their key, so a player can set off the wrong one.`,
    };
  if (o.part === 'look')
    return {
      title: `${a} and ${b} look the same`,
      detail: `Both looks come down to ${o.word}, so from afar they read as one system.`,
    };
  return {
    title: `${a} and ${b} do the same thing`,
    detail: `Both verbs come down to ${o.word}, so they read as one system to a player.`,
  };
}

/**
 * The Overlaps panel of a Game mechanics canvas: each pair of system cards
 * that share a verb, look or key, open ones first, with Show (selects both
 * and brings them into view) and Mark as fine (the person's choice, held
 * until either card's words change).
 */
export function useOverlapsPanel() {
  const overlaps = usePlansStore(s => s.open?.overlaps);
  const nodes = usePlansStore(s => s.open?.plan.nodes);
  const busy = usePlanExtrasStore(s => s.busy);
  const markFine = usePlanExtrasStore(s => s.markFine);
  const setOverlaps = usePlanExtrasStore(s => s.setOverlaps);
  const name = (id: string) =>
    shortName(
      nodes?.find(n => n.id === id),
      id
    ).replace(/^System:\s*/i, '');
  const views: OverlapView[] = (overlaps ?? []).map(o => ({
    key: `${o.a}/${o.b}/${o.part}`,
    a: o.a,
    b: o.b,
    fine: o.fine,
    ...wordsOf(o, name),
  }));
  const open = views.filter(o => !o.fine);
  const fine = views.filter(o => o.fine);
  return {
    open,
    fine,
    // Agrees with the Overlaps button: the open ones, else the ones marked as fine.
    count: open.length || !fine.length ? `${open.length} found` : `${fine.length} marked as fine`,
    busy,
    show: (o: OverlapView) => {
      const plans = usePlansStore.getState();
      usePlansStore.setState({
        nodes: plans.nodes.map(n => ({ ...n, selected: n.id === o.a || n.id === o.b })),
      });
      plans.setArrived([o.a, o.b]);
    },
    markFine: (o: OverlapView) => void markFine(o.a, o.b),
    close: () => setOverlaps(false),
  };
}

/** Whether the Overlaps panel shows: open, and the open canvas a Game mechanics one. */
export function useOverlapsShown() {
  const open = usePlanExtrasStore(s => s.overlaps);
  const mechanics = usePlansStore(s => s.open?.plan.preset === 'mechanics');
  return open && mechanics;
}
