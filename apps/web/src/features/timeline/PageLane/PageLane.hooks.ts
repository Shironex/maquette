import { stopPercent } from '@/state/timeline';
import {
  PAGE_LANES,
  pageAmountLabel,
  pageMarks,
  setPageKey,
  type PageLaneId,
} from '@/state/timeline-page';
import { useTimelineStore } from '@/state/use-timeline-store';

/** The render style or hero text track: the amount at each keyed stop and the fades between. */
export function usePageLane(id: PageLaneId) {
  const view = useTimelineStore(s => s.view);
  const track = useTimelineStore(s => s.draft.timeline[id]);
  const selection = useTimelineStore(s => s.selection);
  const select = useTimelineStore(s => s.select);
  const edit = useTimelineStore(s => s.edit);
  const lane = PAGE_LANES.find(l => l.id === id);
  if (!view || !track || !lane) return null;
  const look = 'look' in track ? track.look : undefined;
  const { keys, spans } = pageMarks(id, track, view.stops);
  const keyed = new Set(keys.map(k => k.stop));
  const active = selection?.kind === 'page' && selection.lane === id;
  const name = (stop: string) => view.stops.find(s => s.id === stop)?.name ?? stop;
  return {
    name: lane.name,
    icon: lane.icon,
    active,
    keys: keys.map(k => ({
      ...k,
      selected: active && selection.stop === k.stop,
      label: `${lane.name} ${pageAmountLabel(id, k.amount, look)} at ${name(k.stop)}`,
    })),
    spans: spans.map(s => ({ ...s, stop: keys.find(k => k.at === s.to)?.stop ?? '' })),
    free: view.stops.flatMap((stop, i) =>
      keyed.has(stop.id)
        ? []
        : [{ id: stop.id, name: stop.name, at: stopPercent(i, view.stops.length) }]
    ),
    select: (stop: string) => select({ kind: 'page', lane: id, stop }),
    add: (stop: string) =>
      edit(d => setPageKey(d, id, stop, id === 'text' ? 0 : 1), {
        kind: 'page',
        lane: id,
        stop,
      }),
  };
}
