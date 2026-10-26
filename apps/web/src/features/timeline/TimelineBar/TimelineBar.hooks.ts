import { useState } from 'react';
import type { MenuItem } from '@/features/common/Menu';
import { addExplode, addPathTrack, type TimelineUnits } from '@/state/timeline';
import { placeLine, unitLabel } from '@/state/timeline-checks';
import { useAppStore } from '@/state/use-app-store';
import { useInspectorStore } from '@/state/use-inspector-store';
import { useTimelineStore } from '@/state/use-timeline-store';
import { usePlayhead } from '../TimelineTracks/TimelineTracks.hooks';

/**
 * What Add track offers: an explode track per model, an energize track per
 * path, the Motion view for states and the Paths editor for a new path.
 */
export function useTrackMenu(): MenuItem[] {
  const view = useTimelineStore(s => s.view);
  const explode = useTimelineStore(s => s.draft.timeline.explode);
  const energized = useTimelineStore(s => s.draft.timeline.paths);
  const edit = useTimelineStore(s => s.edit);
  const setView = useAppStore(s => s.setView);
  if (!view) return [];
  const tracks = view.models
    .filter(m => !explode[m.asset])
    .map(m => ({
      id: `explode-${m.asset}`,
      label: `Explode ${m.name}`,
      icon: 'arrows-out-cardinal' as const,
      disabled: m.parts.length < 2,
      hint:
        m.parts.length < 2
          ? `${m.name} is one part, so there is nothing to spread`
          : 'Spreads its parts apart as the page scrolls',
      onSelect: () => {
        const first = view.stops[0]?.id ?? '';
        edit(d => addExplode(d, m.asset, view.stops), {
          kind: 'explode',
          asset: m.asset,
          stop: first,
        });
      },
    }));
  const first = view.stops[0]?.id ?? '';
  const paths = view.paths
    .filter(p => !energized[p.id])
    .map(p => ({
      id: `path-${p.id}`,
      label: `Energize ${p.name}`,
      icon: 'lightning' as const,
      disabled: view.stops.length === 0,
      hint:
        view.stops.length === 0
          ? 'Add a camera stop first: the fill is keyed at stops'
          : 'Lights it from its first point as the page scrolls',
      onSelect: () =>
        edit(d => addPathTrack(d, p.id, view.stops), { kind: 'path', path: p.id, stop: first }),
    }));
  return [
    ...tracks,
    {
      id: 'states',
      label: 'Object states...',
      icon: 'package',
      hint: 'A prop gets a lane here once it has states in the Motion view',
      onSelect: () => setView('motion'),
    },
    ...paths,
    {
      id: 'new-path',
      label: 'New path...',
      icon: 'path',
      hint: 'Draw a path through the scene in the Inspector, then light it here',
      onSelect: () => {
        useInspectorStore.getState().setTab('paths');
        setView('inspector');
      },
    },
  ];
}

/** Play, the readout, Add track and the ruler's units. */
export function useTimelineBar() {
  const units = useTimelineStore(s => s.units);
  const setUnits = useTimelineStore(s => s.setUnits);
  const stops = useTimelineStore(s => s.view?.stops ?? []);
  const playhead = usePlayhead();
  const [menu, setMenu] = useState(false);
  const items = useTrackMenu();
  const { percent, count, scrollMax } = playhead;
  return {
    playhead,
    units,
    setUnits: (u: TimelineUnits) => setUnits(u),
    canPixels: Boolean(scrollMax),
    percent: unitLabel(percent, 'percent', count, scrollMax),
    detail: [
      scrollMax ? unitLabel(percent, 'pixels', count, scrollMax) : null,
      placeLine(playhead.at, stops),
    ]
      .filter(Boolean)
      .join(' · '),
    menu,
    setMenu,
    items,
  };
}
