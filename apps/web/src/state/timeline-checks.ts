/**
 * What the Timeline view warns about and what it says about the site: the
 * changes that do nothing, ranges of one prop that fight over its parts,
 * ranges that finish off screen, tracks that never move, and what ships.
 */
import type { LayoutStop, TimelineView } from '@/ipc/client';
import {
  explodeMarks,
  pathMarks,
  removeTrigger,
  scrollRange,
  setExplodeKey,
  setPathKey,
  setTrigger,
  stateAt,
  stateMarks,
  stopIndex,
  stopPercent,
  triggerSpan,
  type TimelineDraft,
} from './timeline';
import { PAGE_LANES, addPageTrack, pageMarks } from './timeline-page';

/** A button on a check's card: one edit that clears it. */
export interface TimelineFix {
  label: string;
  tip: string;
  apply: (draft: TimelineDraft) => TimelineDraft;
}

export interface TimelineCheck {
  key: string;
  title: string;
  detail: string;
  fixes?: TimelineFix[];
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
          fixes: [
            {
              label: 'Remove it',
              tip: 'Takes this change off the lane',
              apply: d => removeTrigger(d, asset, mark.index),
            },
          ],
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
          fixes: [
            {
              label: `End ${before.label} at ${Math.round(range.start)}%`,
              tip: 'Shortens the first range so it ends where the next starts',
              apply: d =>
                setTrigger(
                  d,
                  asset,
                  before.index,
                  scrollRange(before.start, Math.max(range.start, before.start + 1))
                ),
            },
          ],
        });
      }
    });
  }
  return out;
}

/** Where the first stop after `start` that does not see `asset` is, up to `end`, in percent. */
function leavesAt(
  stops: readonly LayoutStop[],
  asset: string,
  start: number,
  end: number
): { percent: number; stop: LayoutStop } | null {
  for (const [i, stop] of stops.entries()) {
    const percent = stopPercent(i, stops.length);
    if (percent <= start || percent >= end) continue;
    if (stop.models && !stop.models.includes(asset)) return { percent, stop };
  }
  return null;
}

/**
 * A scroll range that is still moving a prop once the camera has gone to a
 * stop set by hand not to see it. Stops that measure their view say nothing
 * here: the app cannot tell what they see without the page.
 */
function offScreenChecks(view: TimelineView, draft: TimelineDraft): TimelineCheck[] {
  return Object.entries(draft.triggers).flatMap(([asset, triggers]) =>
    (triggers ?? []).flatMap((trigger, index) => {
      const object = view.states[asset];
      if (!object || trigger.kind !== 'scroll') return [];
      const span = triggerSpan(trigger, view.stops);
      const leaves = span && leavesAt(view.stops, asset, span.start, span.end);
      if (!span || !leaves) return [];
      const name = modelName(view, asset);
      const state = object.states.find(s => s.id === trigger.to)?.name ?? trigger.to;
      const end = Math.round(leaves.percent);
      const length = span.end - span.start;
      return [
        {
          key: `off-screen-${asset}-${index}`,
          title: `${name} finishes off screen`,
          detail: `${state} runs to ${Math.round(span.end)}%, but ${leaves.stop.name} at ${end}% does not see ${name}. The end of the move plays out of view.`,
          fixes: [
            {
              label: `End at ${end}%`,
              tip: 'Shortens the range so it ends as the camera leaves',
              apply: (d: TimelineDraft) =>
                setTrigger(d, asset, index, scrollRange(span.start, end)),
            },
            {
              label: 'Move it earlier',
              tip: 'Keeps its length and slides it to end as the camera leaves',
              apply: (d: TimelineDraft) =>
                setTrigger(d, asset, index, scrollRange(Math.max(0, end - length), end)),
            },
          ],
        },
      ];
    })
  );
}

/** Together at the first stop and apart at the last, when there are two stops. */
function spreadFix(view: TimelineView, asset: string): TimelineFix[] {
  const first = view.stops[0]?.id;
  const last = view.stops.at(-1)?.id;
  if (!first || !last || first === last) return [];
  return [
    {
      label: 'Spread by the last stop',
      tip: 'Keys it together at the first stop and apart at the last',
      apply: d => setExplodeKey(setExplodeKey(d, asset, first, 0), asset, last, 1),
    },
  ];
}

/** Lit from end to end at the last stop, keeping the key it has. */
function fillFix(view: TimelineView, path: string): TimelineFix[] {
  const last = view.stops.at(-1);
  if (!last) return [];
  return [
    {
      label: `Light it at ${last.name}`,
      tip: 'Keys it lit from end to end at the last stop',
      apply: d => setPathKey(d, path, last.id, 1),
    },
  ];
}

/** A render style or hero text track that never moves. */
function pageChecks(view: TimelineView, draft: TimelineDraft): TimelineCheck[] {
  return PAGE_LANES.flatMap(lane => {
    const track = draft.timeline[lane.id];
    if (!track || pageMarks(lane.id, track, view.stops).spans.length > 0) return [];
    return [
      {
        key: `still-${lane.id}`,
        title: `${lane.name} never changes`,
        detail: 'Give its track two stops with different amounts.',
        fixes:
          view.stops.length < 2
            ? []
            : [
                {
                  label: 'Key it from first to last',
                  tip: 'Keys the first and the last stop and leaves the rest',
                  apply: (d: TimelineDraft) => addPageTrack(d, lane.id, view.stops),
                },
              ],
      },
    ];
  });
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
        fixes: spreadFix(view, asset),
      },
    ];
  });
}

function pathChecks(view: TimelineView, draft: TimelineDraft): TimelineCheck[] {
  return Object.entries(draft.timeline.paths).flatMap(([id, track]) => {
    if (!track || track.keys.length !== 1 || pathMarks(track, view.stops).spans.length > 0) {
      return [];
    }
    const name = view.paths.find(p => p.id === id)?.name ?? id;
    const amount = track.keys[0]?.amount ?? 0;
    if (amount >= 1) return [];
    return [
      {
        key: `dark-${id}`,
        title: `${name} never fills`,
        detail: 'Give its energize track a second stop with a different amount.',
        fixes: fillFix(view, id),
      },
    ];
  });
}

export function timelineChecks(view: TimelineView, draft: TimelineDraft): TimelineCheck[] {
  return [
    ...stateChecks(view, draft),
    ...offScreenChecks(view, draft),
    ...explodeChecks(view, draft),
    ...pathChecks(view, draft),
    ...pageChecks(view, draft),
  ];
}

/** How many lanes and changes the site gets. */
export function siteSummary(view: TimelineView, draft: TimelineDraft) {
  const stateLanes = Object.keys(draft.triggers).filter(a => view.states[a]);
  const changes = stateLanes.reduce((sum, a) => sum + (draft.triggers[a]?.length ?? 0), 0);
  const tracks = [
    ...Object.values(draft.timeline.explode),
    ...Object.values(draft.timeline.paths),
    draft.timeline.style,
    draft.timeline.text,
  ].filter(Boolean);
  const keys = tracks.reduce((sum, t) => sum + (t?.keys.length ?? 0), 0);
  return {
    lanes: stateLanes.length + tracks.length,
    marks: changes + keys,
  };
}

/** "Between Arrival and Veil gate", "At Arrival", for the playhead's place among the stops. */
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
