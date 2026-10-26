import { useEffect } from 'react';
import { scrollLine, share, stepStop, stopSlots, type BridgeStop } from '@/state/page';
import { leadReading, usePageStore } from '@/state/use-page-store';

/** Milliseconds Play spends on each stop: about the time it takes to read one. */
export const STOP_MS = 4000;

/** Moves the target on at reading speed while Play is on, and stops at the last stop. */
export function usePlayback(count: number) {
  const playing = usePageStore(s => s.playing);
  useEffect(() => {
    if (!playing || count < 2) return;
    let last = performance.now();
    let frame = requestAnimationFrame(function step(now) {
      const { target, seek, setPlaying } = usePageStore.getState();
      const next = Math.min(count - 1, target + (now - last) / STOP_MS);
      last = now;
      seek(next);
      if (next >= count - 1) setPlaying(false);
      else frame = requestAnimationFrame(step);
    });
    return () => cancelAnimationFrame(frame);
  }, [playing, count]);
}

/** The stops, where the page is, and the controls that move it. */
export function useStopScrubber() {
  const readings = usePageStore(s => s.readings);
  const target = usePageStore(s => s.target);
  const playing = usePageStore(s => s.playing);
  const seek = usePageStore(s => s.seek);
  const setPlaying = usePageStore(s => s.setPlaying);
  const cameraStops = usePageStore(s => s.inspect?.stops.stops);
  const lead = leadReading(readings);
  const answered = Boolean(lead.stops);
  const stops: BridgeStop[] = lead.stops ?? (cameraStops ?? []).map(s => ({ name: s.name }));
  const count = stops.length;
  usePlayback(count);
  const at = Math.min(Math.max(0, count - 1), playing ? target : (lead.at ?? target));
  const line = scrollLine(at, stops, lead.scroll);
  const position = share(at, count);
  return {
    stops,
    slots: stopSlots(count),
    count,
    answered,
    at,
    position,
    line,
    playing,
    seek: (stop: number) => {
      setPlaying(false);
      seek(stop);
    },
    jump: (stop: number) => {
      setPlaying(false);
      seek(stop, true);
    },
    prev: () => {
      setPlaying(false);
      seek(stepStop(at, -1, count), true);
    },
    next: () => {
      setPlaying(false);
      seek(stepStop(at, 1, count), true);
    },
    togglePlay: () => {
      if (playing) return setPlaying(false);
      if (at >= count - 1) seek(0);
      setPlaying(true);
    },
  };
}
