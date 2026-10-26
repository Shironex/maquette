/**
 * The Timeline view's rules: the page as it scrolls, one column per camera
 * stop. A stop sits at its share of the page (the site's stop sections are
 * one window high each, so stop i of n is at i / (n - 1)); a prop's states
 * change there through its stop and scroll triggers, and an explode track
 * spreads a model's parts by an amount at each stop. Pure data in, data out.
 */
import type {
  Easing,
  ExplodeKey,
  ExplodeTrack,
  LayoutStop,
  ObjectStates,
  StateTrigger,
  Timeline,
  TimelineView,
} from '@/ipc/client';
import { deepEqual } from '@/lib/deepEqual';
import { n } from './motion';

/** What the view edits: the scroll driven triggers per asset and the explode tracks. */
export interface TimelineDraft {
  triggers: Record<string, StateTrigger[]>;
  timeline: Timeline;
}

export const EMPTY_TIMELINE: TimelineDraft = { triggers: {}, timeline: { explode: {}, paths: {} } };

export type TimelineSelection =
  | { kind: 'state'; asset: string; index: number }
  | { kind: 'explode'; asset: string; stop: string }
  | null;

export type TimelineUnits = 'percent' | 'pixels' | 'stops';

/** A new explode track spreads each part this far, in metres. */
export const DEFAULT_SPREAD = 0.25;
export const MAX_SPREAD = 10;

/** True for the triggers the scroll drives, the ones the timeline shows. */
export const onScroll = (t: StateTrigger) => t.kind === 'stop' || t.kind === 'scroll';

export function draftOf(view: TimelineView): TimelineDraft {
  const triggers: Record<string, StateTrigger[]> = {};
  for (const [asset, object] of Object.entries(view.states)) {
    if (object) triggers[asset] = structuredClone(object.triggers.filter(onScroll));
  }
  return { triggers, timeline: structuredClone(view.timeline) };
}

export function sameDraft(a: TimelineDraft, b: TimelineDraft): boolean {
  return deepEqual(a, b);
}

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/** Where stop `index` of `count` sits, in percent of the page. */
export function stopPercent(index: number, count: number): number {
  return count < 2 ? 0 : (clamp(index, 0, count - 1) / (count - 1)) * 100;
}

/** Percent of the page as fractional stops, what the page bridge's goto takes. */
export function percentToStop(percent: number, count: number): number {
  return count < 2 ? 0 : (clamp(percent, 0, 100) / 100) * (count - 1);
}

export const stopIndex = (stops: readonly LayoutStop[], id: string | null) =>
  stops.findIndex(s => s.id === id);

/** The percent of the page a trigger starts and ends at, or null when its stop is gone. */
export function triggerSpan(
  trigger: StateTrigger,
  stops: readonly LayoutStop[]
): { start: number; end: number } | null {
  if (trigger.kind === 'scroll') return { start: n(trigger.start), end: n(trigger.end) };
  const at = stopIndex(stops, trigger.stop);
  if (at < 0) return null;
  const p = stopPercent(at, stops.length);
  return { start: p, end: p };
}

/** One state change on a lane. */
export interface StateMark {
  index: number;
  kind: 'stop' | 'scroll';
  start: number;
  end: number;
  label: string;
}

const nameOf = (object: ObjectStates, id: string) =>
  object.states.find(s => s.id === id)?.name ?? id;

export function stateMarks(
  object: ObjectStates,
  triggers: readonly StateTrigger[],
  stops: readonly LayoutStop[]
): StateMark[] {
  return triggers.flatMap((t, index) => {
    const span = triggerSpan(t, stops);
    if (!span || (t.kind !== 'stop' && t.kind !== 'scroll')) return [];
    return [{ index, kind: t.kind, ...span, label: nameOf(object, t.to) }];
  });
}

/** The state an object is in at `percent`, from the marks before it (rest before the first). */
export function stateAt(
  object: ObjectStates,
  triggers: readonly StateTrigger[],
  stops: readonly LayoutStop[],
  percent: number
): string | null {
  const marks = stateMarks(object, triggers, stops)
    .filter(m => m.end <= percent)
    .sort((a, b) => a.end - b.end || a.index - b.index);
  const last = marks.at(-1);
  if (last) return triggers[last.index]?.to ?? null;
  return object.states[0]?.id ?? null;
}

/** The state a new mark at `percent` goes to: the next one after where the object is. */
export function nextState(
  object: ObjectStates,
  triggers: readonly StateTrigger[],
  stops: readonly LayoutStop[],
  percent: number
): string | null {
  if (object.states.length < 2) return null;
  const now = stateAt(object, triggers, stops, percent - 1e-6);
  const at = object.states.findIndex(s => s.id === now);
  return object.states[(at + 1) % object.states.length]?.id ?? null;
}

export function addStopMark(
  draft: TimelineDraft,
  asset: string,
  object: ObjectStates,
  stops: readonly LayoutStop[],
  stop: string
): { draft: TimelineDraft; index: number } | null {
  const list = draft.triggers[asset] ?? [];
  const at = stopIndex(stops, stop);
  const to = nextState(object, list, stops, stopPercent(at, stops.length));
  if (at < 0 || !to) return null;
  const trigger: StateTrigger = { kind: 'stop', to, stop, part: null, start: null, end: null };
  const next = { ...draft, triggers: { ...draft.triggers, [asset]: [...list, trigger] } };
  return { draft: next, index: list.length };
}

export function setTrigger(
  draft: TimelineDraft,
  asset: string,
  index: number,
  patch: Partial<StateTrigger>
): TimelineDraft {
  const list = (draft.triggers[asset] ?? []).map((t, i) => (i === index ? { ...t, ...patch } : t));
  return { ...draft, triggers: { ...draft.triggers, [asset]: list } };
}

export function removeTrigger(draft: TimelineDraft, asset: string, index: number): TimelineDraft {
  const list = (draft.triggers[asset] ?? []).filter((_, i) => i !== index);
  return { ...draft, triggers: { ...draft.triggers, [asset]: list } };
}

/**
 * The same change driven the other way: a stop becomes the range from the
 * stop before it to that stop; a range becomes the stop nearest its end.
 */
export function asKind(
  trigger: StateTrigger,
  kind: 'stop' | 'scroll',
  stops: readonly LayoutStop[]
): Partial<StateTrigger> {
  const count = stops.length;
  if (kind === 'scroll') {
    const at = Math.max(0, stopIndex(stops, trigger.stop));
    const end = stopPercent(Math.max(1, at), count) || 100;
    const start = stopPercent(Math.max(0, Math.max(1, at) - 1), count);
    return { kind, stop: null, start: Math.round(start), end: Math.round(end) };
  }
  const near = Math.round(percentToStop(n(trigger.end), count));
  return { kind, stop: stops[near]?.id ?? null, start: null, end: null };
}

/** A scroll range kept in 0 to 100 with its start below its end. */
export function scrollRange(start: number, end: number): { start: number; end: number } {
  const s = clamp(Math.round(start), 0, 99);
  return { start: s, end: clamp(Math.round(end), s + 1, 100) };
}

// Explode tracks.

export interface ExplodeMark {
  stop: string;
  at: number;
  amount: number;
}

export interface ExplodeSpan {
  from: number;
  to: number;
  label: string;
}

/** A track's keys in stop order, with the moves between them. */
export function explodeMarks(
  track: ExplodeTrack,
  stops: readonly LayoutStop[]
): { keys: ExplodeMark[]; spans: ExplodeSpan[] } {
  const keys = track.keys
    .map(k => ({ stop: k.stop, index: stopIndex(stops, k.stop), amount: n(k.amount) }))
    .filter(k => k.index >= 0)
    .sort((a, b) => a.index - b.index)
    .map(k => ({ stop: k.stop, at: stopPercent(k.index, stops.length), amount: k.amount }));
  const spans = keys.slice(1).flatMap((k, i) => {
    const before = keys[i]!;
    if (k.amount === before.amount) return [];
    const label = k.amount > before.amount ? 'Spreads apart' : 'Comes together';
    return [{ from: before.at, to: k.at, label }];
  });
  return { keys, spans };
}

/** "Apart", "Together" or "40% apart". */
export function amountLabel(amount: number): string {
  if (amount >= 1) return 'Apart';
  if (amount <= 0) return 'Together';
  return `${Math.round(amount * 100)}% apart`;
}

const withTrack = (draft: TimelineDraft, asset: string, track: ExplodeTrack | null) => {
  const explode = { ...draft.timeline.explode };
  if (track) explode[asset] = track;
  else delete explode[asset];
  return { ...draft, timeline: { ...draft.timeline, explode } };
};

/** A new track: together at the first stop, apart at the last. */
export function addExplode(
  draft: TimelineDraft,
  asset: string,
  stops: readonly LayoutStop[]
): TimelineDraft {
  const first = stops[0];
  const last = stops.at(-1);
  const keys: ExplodeKey[] = [];
  if (first) keys.push({ stop: first.id, amount: 0 });
  if (last && last !== first) keys.push({ stop: last.id, amount: 1 });
  return withTrack(draft, asset, { distance: DEFAULT_SPREAD, easing: 'ease-in-out', keys });
}

export function removeExplode(draft: TimelineDraft, asset: string): TimelineDraft {
  return withTrack(draft, asset, null);
}

export function setExplode(
  draft: TimelineDraft,
  asset: string,
  patch: { distance?: number; easing?: Easing }
): TimelineDraft {
  const track = draft.timeline.explode[asset];
  if (!track) return draft;
  const distance =
    patch.distance === undefined ? track.distance : clamp(patch.distance, 0.01, MAX_SPREAD);
  return withTrack(draft, asset, { ...track, ...patch, distance });
}

/** Sets the amount at a stop, adding the key; null removes it. */
export function setExplodeKey(
  draft: TimelineDraft,
  asset: string,
  stop: string,
  amount: number | null
): TimelineDraft {
  const track = draft.timeline.explode[asset];
  if (!track) return draft;
  const rest = track.keys.filter(k => k.stop !== stop);
  const keys = amount === null ? rest : [...rest, { stop, amount: clamp(amount, 0, 1) }];
  return withTrack(draft, asset, { ...track, keys });
}
