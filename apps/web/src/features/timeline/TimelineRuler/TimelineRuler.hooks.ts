import { useRef, type KeyboardEvent, type PointerEvent } from 'react';
import { stopPercent, type TimelineUnits } from '@/state/timeline';
import { placeLine, unitLabel } from '@/state/timeline-checks';
import { useTimelineStore } from '@/state/use-timeline-store';
import { percentUnder, usePlayhead } from '../TimelineTracks/TimelineTracks.hooks';

export interface Tick {
  at: number;
  major: boolean;
  label: string | null;
}

/** Ticks every 5 percent with a label every 10; in sections, a number at each stop. */
export function rulerTicks(units: TimelineUnits, count: number, scrollMax: number | null): Tick[] {
  if (units === 'stops') {
    return Array.from({ length: count }, (_, i) => ({
      at: stopPercent(i, count),
      major: true,
      label: String(i + 1),
    }));
  }
  return Array.from({ length: 21 }, (_, i) => {
    const at = i * 5;
    const major = i % 2 === 0;
    const label = major && at > 0 && at < 100 ? unitLabel(at, units, count, scrollMax) : null;
    return { at, major, label };
  });
}

/** How long a key press counts on from where it sent the page, before the page has answered. */
const ANSWER_MS = 800;

/** The ruler the playhead slides on, and the camera stops under it. */
export function useTimelineRuler() {
  const stops = useTimelineStore(s => s.view?.stops ?? []);
  const units = useTimelineStore(s => s.units);
  const playhead = usePlayhead();
  const { count, percent, scrollMax, seekPercent } = playhead;
  const grab = (e: PointerEvent<HTMLDivElement>) => {
    e.currentTarget.setPointerCapture?.(e.pointerId);
    seekPercent(percentUnder(e));
  };
  const scrub = (e: PointerEvent<HTMLDivElement>) => {
    if (e.buttons === 1) seekPercent(percentUnder(e));
  };
  // Keys pressed in a row step on from the last one, not from the page's lagging answer.
  const sent = useRef<{ percent: number; at: number } | null>(null);
  const onKey = (e: KeyboardEvent<HTMLDivElement>) => {
    const last = sent.current;
    const from = last && performance.now() - last.at < ANSWER_MS ? last.percent : percent;
    const by = { ArrowLeft: -1, ArrowRight: 1, PageUp: -10, PageDown: 10 }[e.key];
    const to = by !== undefined ? from + by : e.key === 'Home' ? 0 : e.key === 'End' ? 100 : null;
    if (to === null) return;
    e.preventDefault();
    const next = Math.min(100, Math.max(0, to));
    sent.current = { percent: next, at: performance.now() };
    seekPercent(next);
  };
  const effectiveUnits = units === 'pixels' && !scrollMax ? 'percent' : units;
  return {
    stops: stops.map((stop, i) => ({ id: stop.id, name: stop.name, at: stopPercent(i, count) })),
    ticks: rulerTicks(effectiveUnits, count, scrollMax),
    percent,
    valueText: `${unitLabel(percent, effectiveUnits, count, scrollMax)}, ${placeLine(playhead.at, stops)}`,
    grab,
    scrub,
    onKey,
    jump: playhead.jump,
    current: Math.round(playhead.at),
  };
}
