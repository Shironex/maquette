/**
 * Plays Maquette's scroll timeline from layout.json: a prop's states change
 * where its stop and scroll triggers say, and an explode track spreads a
 * model's parts by the amount keyed at each stop. Pure maths on tour
 * progress; no DOM and no three.js (./tracks.ts moves the nodes).
 *
 * The timeline counts the page in the layout's camera stops, one section
 * each, so stop i of n sits at i / (n - 1) of the page and a scroll range's
 * start and end are percents of that:
 *
 *   "states": {
 *     "switch-cabinet": {
 *       "states": [{ "id": "closed", "name": "Closed", "parts": {} },
 *                  { "id": "open", "name": "Open", "parts": {
 *                    "Door": { "position": [0, 0, 0], "rotation": [0, -105, 0] } } }],
 *       "transitions": [{ "from": "closed", "to": "open", "duration": 0.6,
 *                         "delay": 0, "easing": "ease-out", "stagger": 0 }],
 *       "triggers": [{ "kind": "stop", "to": "open", "stop": "sakura",
 *                      "part": null, "start": null, "end": null }]
 *     }
 *   },
 *   "timeline": {
 *     "explode": { "switch-cabinet": { "distance": 0.3, "easing": "ease-in-out",
 *       "keys": [{ "stop": "canopy", "amount": 0 }, { "stop": "pullback", "amount": 1 }] } }
 *   }
 *
 * Arriving at a stop plays the move once, timed by its transition; a scroll
 * range scrubs it with the page. Click and hover triggers do not play here.
 */
import type { Vec3 } from './manifest';

export type Easing = 'linear' | 'ease-in' | 'ease-out' | 'ease-in-out';
type MaybeVec3 = readonly [number | null, number | null, number | null];

export interface PartPose {
  /** Metres added to the part's own position. */
  position: MaybeVec3;
  /** Degrees added to its own rotation, X, then Y, then Z. */
  rotation: MaybeVec3;
}

export interface ObjectState {
  id: string;
  name: string;
  parts: Record<string, PartPose>;
}

export interface StateTransition {
  from: string;
  to: string;
  duration: number | null;
  delay: number | null;
  easing: Easing;
  stagger: number | null;
}

export interface StateTrigger {
  kind: 'stop' | 'scroll' | 'click' | 'hover';
  to: string;
  stop: string | null;
  part: string | null;
  start: number | null;
  end: number | null;
}

export interface ObjectStates {
  states: ObjectState[];
  transitions: StateTransition[];
  triggers: StateTrigger[];
}

export interface ExplodeTrack {
  distance: number | null;
  easing: Easing;
  keys: { stop: string; amount: number | null }[];
}

export interface Timeline {
  explode: Record<string, ExplodeTrack>;
}

/** A part's offset from rest, plain numbers. */
export interface Offset {
  position: Vec3;
  rotation: Vec3;
}

/** Part name to offset; parts left out are at rest. */
export type Pose = Record<string, Offset>;

/** A move with no transition of its own takes this long, as in Maquette's Motion view. */
export const DEFAULT_MOVE = { duration: 0.6, delay: 0, easing: 'ease-out' as Easing, stagger: 0 };

/** Arriving at a stop: the change plays once progress is this close to it. */
export const ARRIVE = 0.05;

const n = (v: number | null | undefined) => (typeof v === 'number' && Number.isFinite(v) ? v : 0);
const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/** Where an eased move is at `t` (0 to 1), as Maquette eases it. */
export function ease(easing: Easing, t: number): number {
  const x = clamp(t, 0, 1);
  switch (easing) {
    case 'linear':
      return x;
    case 'ease-in':
      return x * x * x;
    case 'ease-out':
      return 1 - (1 - x) ** 3;
    case 'ease-in-out':
      return x < 0.5 ? 4 * x * x * x : 1 - (-2 * x + 2) ** 3 / 2;
  }
}

/** Tour progress as the timeline's percent of the page, for `count` camera stops. */
export function percentOf(progress: number, count: number): number {
  return count < 2 ? 0 : clamp(progress / (count - 1), 0, 1) * 100;
}

/** A state's pose as plain offsets. */
export function poseOf(object: ObjectStates, id: string | undefined): Pose {
  const parts = object.states.find(s => s.id === id)?.parts ?? {};
  const out: Pose = {};
  for (const [name, p] of Object.entries(parts)) {
    out[name] = {
      position: [n(p.position[0]), n(p.position[1]), n(p.position[2])],
      rotation: [n(p.rotation[0]), n(p.rotation[1]), n(p.rotation[2])],
    };
  }
  return out;
}

const ZERO: Offset = { position: [0, 0, 0], rotation: [0, 0, 0] };
const lerp3 = (a: Vec3, b: Vec3, t: number): Vec3 => [
  a[0] + (b[0] - a[0]) * t,
  a[1] + (b[1] - a[1]) * t,
  a[2] + (b[2] - a[2]) * t,
];

/** Each part between pose `a` and pose `b`, at `t` (0 to 1) per part. */
export function mixPose(a: Pose, b: Pose, t: number | ((part: string) => number)): Pose {
  const out: Pose = {};
  for (const name of new Set([...Object.keys(a), ...Object.keys(b)])) {
    const from = a[name] ?? ZERO;
    const to = b[name] ?? ZERO;
    const k = typeof t === 'number' ? t : t(name);
    out[name] = {
      position: lerp3(from.position, to.position, k),
      rotation: lerp3(from.rotation, to.rotation, k),
    };
  }
  return out;
}

/** One scroll driven change, placed on the page. */
export interface StateEvent {
  index: number;
  kind: 'stop' | 'scroll';
  to: string;
  start: number;
  end: number;
}

/** The stop and scroll triggers in page order; a stop the layout lost drops its change. */
export function stateEvents(object: ObjectStates, stopIds: readonly string[]): StateEvent[] {
  const count = stopIds.length;
  return object.triggers
    .flatMap((t, index): StateEvent[] => {
      if (t.kind === 'scroll') {
        return [{ index, kind: 'scroll', to: t.to, start: n(t.start), end: n(t.end) }];
      }
      if (t.kind !== 'stop') return [];
      const at = stopIds.indexOf(t.stop ?? '');
      if (at < 0) return [];
      const p = count < 2 ? 0 : (at / (count - 1)) * 100;
      return [{ index, kind: 'stop', to: t.to, start: p, end: p }];
    })
    .sort((a, b) => a.end - b.end || a.index - b.index);
}

/** The move from one state to another: its transition, or Maquette's default. */
export function moveOf(object: ObjectStates, from: string | undefined, to: string) {
  const t = object.transitions.find(m => m.from === from && m.to === to);
  return {
    duration: Math.max(0, t?.duration ?? DEFAULT_MOVE.duration),
    delay: Math.max(0, t?.delay ?? DEFAULT_MOVE.delay),
    easing: t?.easing ?? DEFAULT_MOVE.easing,
    stagger: Math.max(0, t?.stagger ?? DEFAULT_MOVE.stagger),
  };
}

/**
 * Where the scroll puts an object at `percent`: the changes that are done,
 * with a scroll range under way scrubbed by its easing. Stop changes count
 * as done the moment the camera arrives; the animator times them.
 */
export function settleAt(
  object: ObjectStates,
  events: readonly StateEvent[],
  percent: number,
  arrive = 0
): { pose: Pose; state: string | undefined; done: number[] } {
  let state = object.states[0]?.id;
  let pose = poseOf(object, state);
  const done: number[] = [];
  for (const e of events) {
    const reached = e.kind === 'stop' ? percent >= e.end - arrive : percent >= e.end;
    if (reached) {
      state = e.to;
      pose = poseOf(object, state);
      done.push(e.index);
      continue;
    }
    if (e.kind === 'scroll' && percent > e.start) {
      const t = ease(moveOf(object, state, e.to).easing, (percent - e.start) / (e.end - e.start));
      pose = mixPose(pose, poseOf(object, e.to), t);
    }
    break;
  }
  return { pose, state, done };
}

/**
 * Plays an object's states as the page scrolls. A scroll range follows the
 * page; arriving at (or leaving) a stop change plays its move over time,
 * part by part with its stagger, from wherever the parts are.
 */
export function stateAnimator(object: ObjectStates, stopIds: readonly string[]) {
  const events = stateEvents(object, stopIds);
  const count = stopIds.length;
  const arrive = count < 2 ? 0 : (ARRIVE / (count - 1)) * 100;
  let last: { done: Set<number>; state: string | undefined } | null = null;
  let shown: Pose = poseOf(object, object.states[0]?.id);
  let tween: {
    from: Pose;
    start: number;
    move: ReturnType<typeof moveOf>;
    order: string[];
  } | null = null;

  return {
    /** The pose at tour progress `progress` and clock time `now` (seconds). */
    update(progress: number, now: number): { pose: Pose; moving: boolean } {
      const goal = settleAt(object, events, percentOf(progress, count), arrive);
      const done = new Set(goal.done);
      const before = last;
      const byStop =
        before !== null &&
        events.some(e => e.kind === 'stop' && before.done.has(e.index) !== done.has(e.index));
      if (byStop) {
        const move = moveOf(object, before.state, goal.state ?? '');
        tween = { from: shown, start: now, move, order: Object.keys(mixPose(shown, goal.pose, 0)) };
      }
      last = { done, state: goal.state };
      if (!tween) {
        shown = goal.pose;
        return { pose: shown, moving: false };
      }
      const { from, start, move, order } = tween;
      const local = (part: string) => {
        const offset = move.delay + Math.max(0, order.indexOf(part)) * move.stagger;
        const t = move.duration > 0 ? (now - start - offset) / move.duration : 1;
        return ease(move.easing, t);
      };
      shown = mixPose(from, goal.pose, local);
      const end = move.delay + Math.max(0, order.length - 1) * move.stagger + move.duration;
      if (now - start >= end) tween = null;
      return { pose: shown, moving: tween !== null };
    },
  };
}

/**
 * How far apart an explode track has the parts at tour progress `progress`,
 * 0 to 1: eased between the keyed stops, held before the first and after
 * the last.
 */
export function explodeAt(track: ExplodeTrack, stopIds: readonly string[], progress: number) {
  const keys = track.keys
    .map(k => ({ at: stopIds.indexOf(k.stop), amount: clamp(n(k.amount), 0, 1) }))
    .filter(k => k.at >= 0)
    .sort((a, b) => a.at - b.at);
  const first = keys[0];
  if (!first) return 0;
  if (progress <= first.at) return first.amount;
  for (let i = 1; i < keys.length; i += 1) {
    const a = keys[i - 1]!;
    const b = keys[i]!;
    if (progress <= b.at) {
      const t = ease(track.easing, (progress - a.at) / (b.at - a.at));
      return a.amount + (b.amount - a.amount) * t;
    }
  }
  return keys.at(-1)!.amount;
}
