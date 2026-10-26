import { addStopMark, stateMarks, stopPercent } from '@/state/timeline';
import { useTimelineStore } from '@/state/use-timeline-store';

/** One prop's lane: its state changes along the page, and a place to add one at each stop. */
export function useStateLane(asset: string) {
  const view = useTimelineStore(s => s.view);
  const triggers = useTimelineStore(s => s.draft.triggers[asset]);
  const selection = useTimelineStore(s => s.selection);
  const select = useTimelineStore(s => s.select);
  const edit = useTimelineStore(s => s.edit);
  const object = view?.states[asset];
  const model = view?.models.find(m => m.asset === asset);
  if (!view || !object || !model) return null;
  const stops = view.stops;
  const list = triggers ?? [];
  const marks = stateMarks(object, list, stops);
  const taken = new Set(list.filter(t => t.kind === 'stop').map(t => t.stop));
  const selected =
    selection?.kind === 'state' && selection.asset === asset ? selection.index : null;
  return {
    name: model.name,
    marks: marks.map(m => ({ ...m, selected: m.index === selected })),
    free: stops.flatMap((stop, i) =>
      taken.has(stop.id) ? [] : [{ id: stop.id, name: stop.name, at: stopPercent(i, stops.length) }]
    ),
    canAdd: object.states.length > 1,
    active: selection?.asset === asset,
    select: (index: number) => select({ kind: 'state', asset, index }),
    add: (stop: string) => {
      const added = addStopMark(useTimelineStore.getState().draft, asset, object, stops, stop);
      if (added) edit(() => added.draft, { kind: 'state', asset, index: added.index });
    },
  };
}
