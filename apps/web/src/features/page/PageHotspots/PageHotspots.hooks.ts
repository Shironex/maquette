import { useEffect, useState } from 'react';
import { commands, type SceneLayout } from '@/ipc/client';
import { device } from '@/state/page';
import { hotspotCount, hotspotsAt } from '@/state/page-hotspots';
import { leadReading, usePageStore } from '@/state/use-page-store';

/** The props' click and hover triggers at the playhead, read from the layout when shown. */
export function usePageHotspots() {
  const project = usePageStore(s => s.project);
  const deviceId = usePageStore(s => s.device);
  const target = usePageStore(s => s.target);
  const playing = usePageStore(s => s.playing);
  const readings = usePageStore(s => s.readings);
  const [layout, setLayout] = useState<SceneLayout | null>(null);
  useEffect(() => {
    if (!project) return;
    let live = true;
    commands
      .layoutGet(project)
      .then(view => {
        if (live) setLayout(view.layout);
      })
      .catch(() => {
        if (live) setLayout(null);
      });
    return () => {
      live = false;
    };
  }, [project]);
  const d = device(deviceId);
  const at = playing ? target : (leadReading(readings).at ?? target);
  const spots = layout ? hotspotsAt(layout, at, d.width / d.height) : [];
  const total = layout ? hotspotCount(layout) : 0;
  const line =
    !layout || spots.length > 0
      ? null
      : total === 0
        ? 'No placed prop has a click or hover trigger yet'
        : `${total} ${total === 1 ? 'hotspot is' : 'hotspots are'} out of view at this stop`;
  return { spots, line };
}
