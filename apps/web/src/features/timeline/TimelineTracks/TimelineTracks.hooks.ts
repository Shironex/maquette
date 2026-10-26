import { useEffect, useRef, type PointerEvent } from 'react';
import { typing } from '@/lib/typing';
import { stepStop } from '@/state/page';
import { percentToStop, stopPercent } from '@/state/timeline';
import { leadReading, usePageStore } from '@/state/use-page-store';
import { useTimelineStore } from '@/state/use-timeline-store';

/**
 * Where the page is, in percent of its scroll, and the controls that move
 * it. The playhead is the Page view's: a seek goes to the page preview
 * through the page bridge, and what the page answers moves the playhead.
 */
export function usePlayhead() {
  const count = useTimelineStore(s => s.view?.stops.length ?? 0);
  const readings = usePageStore(s => s.readings);
  const target = usePageStore(s => s.target);
  const playing = usePageStore(s => s.playing);
  const seek = usePageStore(s => s.seek);
  const setPlaying = usePageStore(s => s.setPlaying);
  const lead = leadReading(readings);
  const last = Math.max(0, count - 1);
  const at = Math.min(last, Math.max(0, playing ? target : (lead.at ?? target)));
  const move = (stop: number, smooth: boolean) => {
    setPlaying(false);
    seek(stop, smooth);
  };
  return {
    at,
    percent: stopPercent(at, count),
    count,
    playing,
    scrollMax: lead.scroll?.max ?? null,
    seekPercent: (percent: number) => move(percentToStop(percent, count), false),
    jump: (stop: number) => move(stop, true),
    step: (dir: 1 | -1) => move(stepStop(at, dir, count), true),
    top: () => move(0, true),
    togglePlay: () => {
      if (playing) return setPlaying(false);
      if (at >= last) seek(0);
      setPlaying(true);
    },
  };
}

/** The percent of the page under a pointer on the ruler. */
export function percentUnder(e: PointerEvent<HTMLElement>): number {
  const box = e.currentTarget.getBoundingClientRect();
  const share = box.width > 0 ? (e.clientX - box.left) / box.width : 0;
  return Math.min(Math.max(share, 0), 1) * 100;
}

/** The lanes, the playhead and the keys: Space plays, [ and ] jump a stop. */
export function useTimelineTracks() {
  const view = useTimelineStore(s => s.view);
  const draft = useTimelineStore(s => s.draft);
  const playhead = usePlayhead();
  const latest = useRef(playhead);
  latest.current = playhead;
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (typing(e.target) || e.metaKey || e.ctrlKey || e.altKey) return;
      const p = latest.current;
      const keys: Record<string, () => void> = {
        ' ': p.togglePlay,
        '[': () => p.step(-1),
        ']': () => p.step(1),
      };
      const run = keys[e.key];
      if (!run) return;
      e.preventDefault();
      run();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);
  const states = (view?.models ?? []).filter(m => view?.states[m.asset]);
  const explode = (view?.models ?? []).filter(m => draft.timeline.explode[m.asset]);
  const paths = (view?.paths ?? []).filter(p => draft.timeline.paths[p.id]);
  return { view, states, explode, paths, playhead };
}
