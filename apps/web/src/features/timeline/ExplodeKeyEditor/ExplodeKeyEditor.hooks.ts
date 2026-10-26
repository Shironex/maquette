import type { Easing } from '@/ipc/client';
import { n } from '@/state/motion';
import { EASINGS } from '@/state/motion-states';
import { MAX_SPREAD, removeExplode, setExplode, setExplodeKey } from '@/state/timeline';
import { useTimelineStore } from '@/state/use-timeline-store';

/** The selected explode key and its track: how far apart at this stop, how far and how it eases. */
export function useExplodeKeyEditor() {
  const view = useTimelineStore(s => s.view);
  const track = useTimelineStore(s =>
    s.selection?.kind === 'explode' ? s.draft.timeline.explode[s.selection.asset] : undefined
  );
  const selection = useTimelineStore(s => s.selection);
  const edit = useTimelineStore(s => s.edit);
  if (!view || selection?.kind !== 'explode' || !track) return null;
  const { asset, stop } = selection;
  const model = view.models.find(m => m.asset === asset);
  const stopName = view.stops.find(s => s.id === stop)?.name ?? stop;
  const key = track.keys.find(k => k.stop === stop);
  const parts = model?.parts.length ?? 0;
  return {
    title: `${model?.name ?? asset} · explode at ${stopName}`,
    stopName,
    keyed: Boolean(key),
    amount: Math.round((key?.amount ?? 0) * 100),
    distance: n(track.distance),
    easing: track.easing,
    easings: EASINGS,
    maxSpread: MAX_SPREAD,
    facts: [
      { label: 'Parts that move', value: parts > 0 ? String(parts) : 'Not read yet' },
      { label: 'Between stops', value: 'Eases with the scroll' },
    ],
    setAmount: (percent: number) => edit(d => setExplodeKey(d, asset, stop, percent / 100)),
    setDistance: (distance: number) => edit(d => setExplode(d, asset, { distance })),
    setEasing: (easing: Easing) => edit(d => setExplode(d, asset, { easing })),
    removeKey: () => edit(d => setExplodeKey(d, asset, stop, null)),
    removeTrack: () => edit(d => removeExplode(d, asset), null),
  };
}
