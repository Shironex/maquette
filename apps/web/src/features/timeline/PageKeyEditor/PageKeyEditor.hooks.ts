import type { Easing, TrackLook } from '@/ipc/client';
import { EASINGS } from '@/state/motion-states';
import {
  LOOKS,
  PAGE_LANES,
  removePageTrack,
  setLook,
  setPageEasing,
  setPageKey,
} from '@/state/timeline-page';
import { useTimelineStore } from '@/state/use-timeline-store';

/** The selected render style or hero text key and its track. */
export function usePageKeyEditor() {
  const view = useTimelineStore(s => s.view);
  const selection = useTimelineStore(s => s.selection);
  const draft = useTimelineStore(s => s.draft);
  const edit = useTimelineStore(s => s.edit);
  if (!view || selection?.kind !== 'page') return null;
  const { lane, stop } = selection;
  const track = draft.timeline[lane];
  const info = PAGE_LANES.find(l => l.id === lane);
  if (!track || !info) return null;
  const stopName = view.stops.find(s => s.id === stop)?.name ?? stop;
  const key = track.keys.find(k => k.stop === stop);
  const style = lane === 'style';
  return {
    title: `${info.name} at ${stopName}`,
    style,
    stopName,
    keyed: Boolean(key),
    amount: Math.round((key?.amount ?? 0) * 100),
    amountLabel: style ? `Look at ${stopName}` : `Shown at ${stopName}`,
    look: draft.timeline.style?.look ?? 'blueprint',
    looks: LOOKS,
    easing: track.easing,
    easings: EASINGS,
    facts: style
      ? [
          { label: 'On the site', value: 'A filter on the 3D layer' },
          { label: 'Between stops', value: 'Fades with the scroll' },
          { label: 'Draw calls', value: 'None added' },
        ]
      : [
          { label: 'Fades', value: 'Copy marked data-maquette-text' },
          { label: 'Between stops', value: 'Fades with the scroll' },
          { label: 'Without 3D', value: 'The copy stays' },
        ],
    setAmount: (percent: number) => edit(d => setPageKey(d, lane, stop, percent / 100)),
    setLook: (look: TrackLook) => edit(d => setLook(d, look)),
    setEasing: (easing: Easing) => edit(d => setPageEasing(d, lane, easing)),
    removeKey: () => edit(d => setPageKey(d, lane, stop, null)),
    removeTrack: () => edit(d => removePageTrack(d, lane), null),
  };
}
