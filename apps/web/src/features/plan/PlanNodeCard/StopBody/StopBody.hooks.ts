import type { PlanNode } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { lineOf, placesOf } from '@/state/plan-camera';
import { usePlanCameraStore } from '@/state/use-plan-camera-store';

/** Where a proposal stop sits against its place in the main line. */
function movedNote(from: number, at: number): string {
  return from === at ? `still stop ${from}` : `moved from stop ${from}`;
}

/**
 * What a stop node shows beside its stop: its place in the main line, or
 * the proposal it belongs to and the place the stop has in the main line
 * now; its poster render when the Capture posters step made one.
 */
export function useStopBody(node: PlanNode) {
  const view = usePlanCameraStore(s => s.view);
  const stop = node.stop;
  const line = lineOf(view, node.id);
  const places = placesOf(view);
  const place = places.get(node.id) ?? null;
  const poster = view?.posters.find(p => p.stop === stop?.id)?.poster ?? null;
  let note = '';
  if (line?.main && place === 1) note = 'first view';
  else if (line?.main && place === line.nodes.length) note = 'last';
  else if (line && !line.main && stop && view) {
    const doc = view.doc.plan.nodes;
    const was = view.main.findIndex(id => doc.find(n => n.id === id)?.stop?.id === stop.id);
    const at = line.nodes.indexOf(node.id) + 1;
    note = was < 0 ? 'new angle' : movedNote(was + 1, at);
  }
  const at = line ? line.nodes.indexOf(node.id) + 1 : 0;
  return {
    stop,
    src: fileUrl(poster),
    /** "Stop 2" on the main line, "Proposal 3" on an agent's, "Set aside" alone. */
    label: line?.main
      ? `Stop ${place}`
      : line && line.nodes.length > 1
        ? `Proposal ${at}`
        : 'Set aside',
    main: Boolean(line?.main),
    note,
  };
}
