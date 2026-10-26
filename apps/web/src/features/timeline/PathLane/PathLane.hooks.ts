import { litLabel, pathMarks, setPathKey, stopPercent } from '@/state/timeline';
import { useTimelineStore } from '@/state/use-timeline-store';

/** One path's energize track: how much of it is lit at each keyed stop. */
export function usePathLane(id: string) {
  const view = useTimelineStore(s => s.view);
  const track = useTimelineStore(s => s.draft.timeline.paths[id]);
  const selection = useTimelineStore(s => s.selection);
  const select = useTimelineStore(s => s.select);
  const edit = useTimelineStore(s => s.edit);
  const path = view?.paths.find(p => p.id === id);
  if (!view || !track || !path) return null;
  const { keys, spans } = pathMarks(track, view.stops);
  const keyed = new Set(keys.map(k => k.stop));
  const selected = selection?.kind === 'path' && selection.path === id ? selection.stop : null;
  const name = (stop: string) => view.stops.find(s => s.id === stop)?.name ?? stop;
  return {
    name: path.name,
    glow: path.glow,
    keys: keys.map(k => ({
      ...k,
      selected: k.stop === selected,
      label: `${path.name} ${litLabel(k.amount).toLowerCase()} at ${name(k.stop)}`,
    })),
    spans: spans.map(s => ({
      ...s,
      stop: keys.find(k => k.at === s.to)?.stop ?? '',
    })),
    free: view.stops.flatMap((stop, i) =>
      keyed.has(stop.id)
        ? []
        : [{ id: stop.id, name: stop.name, at: stopPercent(i, view.stops.length) }]
    ),
    active: selection?.kind === 'path' && selection.path === id,
    select: (stop: string) => select({ kind: 'path', path: id, stop }),
    add: (stop: string) => edit(d => setPathKey(d, id, stop, 1), { kind: 'path', path: id, stop }),
  };
}
