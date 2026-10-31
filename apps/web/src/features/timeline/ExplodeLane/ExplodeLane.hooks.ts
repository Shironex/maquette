import { amountLabel, explodeMarks, setExplodeKey, stopPercent } from '@/state/timeline';
import { useTimelineStore } from '@/state/use-timeline-store';

/** One model's explode track: how far apart its parts are at each keyed stop. */
export function useExplodeLane(asset: string) {
  const view = useTimelineStore(s => s.view);
  const track = useTimelineStore(s => s.draft.timeline.explode[asset]);
  const selection = useTimelineStore(s => s.selection);
  const select = useTimelineStore(s => s.select);
  const edit = useTimelineStore(s => s.edit);
  const model = view?.models.find(m => m.asset === asset);
  if (!view || !track || !model) return null;
  const { keys, spans } = explodeMarks(track, view.stops);
  const keyed = new Set(keys.map(k => k.stop));
  const selected =
    selection?.kind === 'explode' && selection.asset === asset ? selection.stop : null;
  const name = (stop: string) => view.stops.find(s => s.id === stop)?.name ?? stop;
  return {
    name: model.name,
    keys: keys.map(k => ({
      ...k,
      selected: k.stop === selected,
      label: `${model.name} ${amountLabel(k.amount).toLowerCase()} at ${name(k.stop)}`,
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
    active:
      (selection?.kind === 'state' || selection?.kind === 'explode') && selection.asset === asset,
    select: (stop: string) => select({ kind: 'explode', asset, stop }),
    add: (stop: string) =>
      edit(d => setExplodeKey(d, asset, stop, 1), { kind: 'explode', asset, stop }),
  };
}
