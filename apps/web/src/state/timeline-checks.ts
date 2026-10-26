/**
 * What the Timeline view warns about and what it says about the site: the
 * changes that do nothing, ranges of one prop that fight over its parts,
 * explode tracks that never move, and what ships.
 */
import type { LayoutStop, TimelineView } from '@/ipc/client';
import { explodeMarks, stateAt, stateMarks, stopIndex, type TimelineDraft } from './timeline';

export interface TimelineCheck {
  key: string;
  title: string;
  detail: string;
}

const modelName = (view: TimelineView, asset: string) =>
  view.models.find(m => m.asset === asset)?.name ?? asset;

const stopName = (stops: readonly LayoutStop[], id: string | null) =>
  stops[stopIndex(stops, id)]?.name ?? id ?? 'a stop';

function stateChecks(view: TimelineView, draft: TimelineDraft): TimelineCheck[] {
  const out: TimelineCheck[] = [];
  for (const [asset, triggers] of Object.entries(draft.triggers)) {
    const object = view.states[asset];
    if (!object || !triggers) continue;
    const name = modelName(view, asset);
    const marks = stateMarks(object, triggers, view.stops);
    for (const mark of marks) {
      const trigger = triggers[mark.index]!;
      if (trigger.kind !== 'stop') continue;
      const before = stateAt(
        object,
        triggers.filter((_, i) => i !== mark.index),
        view.stops,
        mark.start
      );
      if (before === trigger.to) {
        out.push({
          key: `same-${asset}-${mark.index}`,
          title: `No change at ${stopName(view.stops, trigger.stop)}`,
          detail: `${name} is already ${mark.label} there. Pick another state or remove it.`,
        });
      }
    }
    const ranges = marks.filter(m => m.kind === 'scroll').sort((a, b) => a.start - b.start);
    ranges.slice(1).forEach((range, i) => {
      const before = ranges[i]!;
      if (range.start < before.end) {
        out.push({
          key: `overlap-${asset}-${range.index}`,
          title: `Two ranges of ${name} overlap`,
          detail: `${before.label} runs to ${Math.round(before.end)}% and ${range.label} starts at ${Math.round(range.start)}%. Let one end before the next starts.`,
        });
      }
    });
  }
  return out;
}

function explodeChecks(view: TimelineView, draft: TimelineDraft): TimelineCheck[] {
  return Object.entries(draft.timeline.explode).flatMap(([asset, track]) => {
    if (!track) return [];
    const { spans } = explodeMarks(track, view.stops);
    if (spans.length > 0) return [];
    return [
      {
        key: `still-${asset}`,
        title: `${modelName(view, asset)} never spreads`,
        detail: 'Give its explode track two stops with different amounts.',
      },
    ];
  });
}

export function timelineChecks(view: TimelineView, draft: TimelineDraft): TimelineCheck[] {
  return [...stateChecks(view, draft), ...explodeChecks(view, draft)];
}

/** How many lanes and changes the site gets. */
export function siteSummary(view: TimelineView, draft: TimelineDraft) {
  const stateLanes = Object.keys(draft.triggers).filter(a => view.states[a]);
  const changes = stateLanes.reduce((sum, a) => sum + (draft.triggers[a]?.length ?? 0), 0);
  const tracks = Object.values(draft.timeline.explode).filter(Boolean);
  const keys = tracks.reduce((sum, t) => sum + (t?.keys.length ?? 0), 0);
  return {
    lanes: stateLanes.length + tracks.length,
    marks: changes + keys,
  };
}

/** "Between Porch and Sakura", "At Porch", for the playhead's place among the stops. */
export function placeLine(at: number, stops: readonly LayoutStop[]): string {
  if (stops.length === 0) return '';
  const index = Math.min(stops.length - 1, Math.max(0, at));
  const low = Math.floor(index + 1e-3);
  const high = Math.ceil(index - 1e-3);
  if (low >= high) return `at ${stops[low]?.name ?? ''}`;
  return `between ${stops[low]?.name ?? ''} and ${stops[high]?.name ?? ''}`;
}

/** A percent of the page in the chosen units: "46%", "2,740 px" or "stop 2.4". */
export function unitLabel(
  percent: number,
  units: 'percent' | 'pixels' | 'stops',
  count: number,
  scrollMax: number | null
): string {
  if (units === 'pixels' && scrollMax) {
    return `${Math.round((percent / 100) * scrollMax).toLocaleString('en-US')} px`;
  }
  if (units === 'stops') {
    const stop = count < 2 ? 1 : (percent / 100) * (count - 1) + 1;
    return `stop ${Math.round(stop * 10) / 10}`;
  }
  return `${Math.round(percent)}%`;
}
