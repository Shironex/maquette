import type { Easing } from '@/ipc/client';
import { EASINGS } from '@/state/motion-states';
import { pathLength } from '@/state/paths';
import { removePathTrack, setPathEasing, setPathKey } from '@/state/timeline';
import { useAppStore } from '@/state/use-app-store';
import { useInspectorStore } from '@/state/use-inspector-store';
import { usePathsStore } from '@/state/use-paths-store';
import { useTimelineStore } from '@/state/use-timeline-store';

/** The selected energize key and its track: how much is lit at this stop and how it eases. */
export function usePathKeyEditor() {
  const view = useTimelineStore(s => s.view);
  const track = useTimelineStore(s =>
    s.selection?.kind === 'path' ? s.draft.timeline.paths[s.selection.path] : undefined
  );
  const selection = useTimelineStore(s => s.selection);
  const edit = useTimelineStore(s => s.edit);
  const setView = useAppStore(s => s.setView);
  if (!view || selection?.kind !== 'path' || !track) return null;
  const { path: id, stop } = selection;
  const path = view.paths.find(p => p.id === id);
  const stopName = view.stops.find(s => s.id === stop)?.name ?? stop;
  const key = track.keys.find(k => k.stop === stop);
  return {
    title: `${path?.name ?? id} · energize at ${stopName}`,
    stopName,
    keyed: Boolean(key),
    amount: Math.round((key?.amount ?? 0) * 100),
    easing: track.easing,
    easings: EASINGS,
    facts: [
      { label: 'Length', value: path ? `${pathLength(path).toFixed(1)} m` : 'Gone' },
      { label: 'Lights', value: 'From its first point to its last' },
      { label: 'Between stops', value: 'Fills with the scroll' },
    ],
    setAmount: (percent: number) => edit(d => setPathKey(d, id, stop, percent / 100)),
    setEasing: (easing: Easing) => edit(d => setPathEasing(d, id, easing)),
    removeKey: () => edit(d => setPathKey(d, id, stop, null)),
    removeTrack: () => edit(d => removePathTrack(d, id), null),
    openPath: () => {
      usePathsStore.getState().select({ path: id, point: null });
      useInspectorStore.getState().setTab('paths');
      setView('inspector');
    },
  };
}
