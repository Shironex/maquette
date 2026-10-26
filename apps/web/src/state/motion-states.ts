/**
 * Object states in the Motion draft: adding and naming states, posing their
 * parts, timing the transitions between them and the triggers that start
 * them. Plain functions over one asset's `ObjectStates`; the layout checks
 * the same rules again on save.
 */
import type {
  Easing,
  LayoutStop,
  ObjectState,
  ObjectStates,
  PartPose,
  StateTransition,
  StateTrigger,
  TriggerKind,
} from '@/ipc/client';
import { n } from './motion';

export const MAX_STATES = 16;
export const MAX_TRIGGERS = 32;
export const DEFAULT_DURATION = 0.6;
export const EASINGS: readonly { value: Easing; label: string }[] = [
  { value: 'ease-out', label: 'Ease out, cubic' },
  { value: 'ease-in', label: 'Ease in, cubic' },
  { value: 'ease-in-out', label: 'Ease in and out' },
  { value: 'linear', label: 'Linear' },
];

export type Vec3 = [number, number, number];

export interface PlainPose {
  position: Vec3;
  rotation: Vec3;
}

export const REST: PlainPose = { position: [0, 0, 0], rotation: [0, 0, 0] };

const vec = (v: readonly (number | null)[]): Vec3 => [n(v[0]), n(v[1]), n(v[2])];

export function poseOf(pose: PartPose | undefined): PlainPose {
  return pose ? { position: vec(pose.position), rotation: vec(pose.rotation) } : REST;
}

export function isRest(pose: PlainPose): boolean {
  return [...pose.position, ...pose.rotation].every(v => Math.abs(v) < 1e-6);
}

/** "Door open" to "door-open", unique among `taken`. */
export function stateId(name: string, taken: readonly string[]): string {
  const base =
    name
      .toLowerCase()
      .normalize('NFKD')
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '') || 'state';
  let id = base;
  for (let i = 2; taken.includes(id); i += 1) id = `${base}-${i}`;
  return id;
}

/** A first state for a model with none: its rest pose. */
export function firstStates(): ObjectStates {
  return {
    states: [{ id: 'rest', name: 'Rest', parts: {} }],
    transitions: [],
    triggers: [],
  };
}

/** A new state posed like `from`, with a transition into it from the one before. */
export function addState(object: ObjectStates, name: string, from?: string): ObjectStates {
  if (object.states.length >= MAX_STATES) return object;
  const id = stateId(
    name,
    object.states.map(s => s.id)
  );
  const source = object.states.find(s => s.id === from) ?? object.states.at(-1);
  const state: ObjectState = { id, name, parts: structuredClone(source?.parts ?? {}) };
  const transitions = source
    ? [...object.transitions, transition(source.id, id)]
    : object.transitions;
  return { ...object, states: [...object.states, state], transitions };
}

export function transition(from: string, to: string): StateTransition {
  return { from, to, duration: DEFAULT_DURATION, delay: 0, easing: 'ease-out', stagger: 0 };
}

export function renameState(object: ObjectStates, id: string, name: string): ObjectStates {
  return { ...object, states: object.states.map(s => (s.id === id ? { ...s, name } : s)) };
}

/** Drops a state with its transitions and triggers; the rest pose stays. */
export function removeState(object: ObjectStates, id: string): ObjectStates {
  if (object.states[0]?.id === id) return object;
  return {
    states: object.states.filter(s => s.id !== id),
    transitions: object.transitions.filter(t => t.from !== id && t.to !== id),
    triggers: object.triggers.filter(t => t.to !== id),
  };
}

/** Sets how `part` moves in state `id`; a pose at rest drops the part. */
export function setPose(
  object: ObjectStates,
  id: string,
  part: string,
  pose: PlainPose
): ObjectStates {
  return {
    ...object,
    states: object.states.map(s => {
      if (s.id !== id) return s;
      const parts = { ...s.parts };
      if (isRest(pose)) delete parts[part];
      else parts[part] = { position: [...pose.position], rotation: [...pose.rotation] };
      return { ...s, parts };
    }),
  };
}

export function setTransition(
  object: ObjectStates,
  index: number,
  patch: Partial<Omit<StateTransition, 'from' | 'to'>>
): ObjectStates {
  return {
    ...object,
    transitions: object.transitions.map((t, i) => (i === index ? { ...t, ...patch } : t)),
  };
}

/** The transition into `to`, preferring the one from `from`. */
export function transitionInto(
  object: ObjectStates,
  to: string,
  from?: string
): { transition: StateTransition; index: number } | null {
  const exact = from ? object.transitions.findIndex(t => t.to === to && t.from === from) : -1;
  const index = exact >= 0 ? exact : object.transitions.findIndex(t => t.to === to);
  const found = object.transitions[index];
  return found ? { transition: found, index } : null;
}

/** The parts a transition moves: the ones posed differently at either end. */
export function partsInMove(object: ObjectStates, t: StateTransition): string[] {
  const from = object.states.find(s => s.id === t.from)?.parts ?? {};
  const to = object.states.find(s => s.id === t.to)?.parts ?? {};
  const names = new Set([...Object.keys(from), ...Object.keys(to)]);
  return [...names].filter(p => JSON.stringify(poseOf(from[p])) !== JSON.stringify(poseOf(to[p])));
}

export function newTrigger(kind: TriggerKind, to: string, stops: readonly LayoutStop[]) {
  const trigger: StateTrigger = { kind, to, stop: null, part: null, start: null, end: null };
  if (kind === 'stop') trigger.stop = stops[0]?.id ?? null;
  if (kind === 'scroll') {
    trigger.start = 40;
    trigger.end = 55;
  }
  return trigger;
}

export function addTrigger(object: ObjectStates, trigger: StateTrigger): ObjectStates {
  if (object.triggers.length >= MAX_TRIGGERS) return object;
  return { ...object, triggers: [...object.triggers, trigger] };
}

export function removeTrigger(object: ObjectStates, index: number): ObjectStates {
  return { ...object, triggers: object.triggers.filter((_, i) => i !== index) };
}

const stateName = (object: ObjectStates, id: string) =>
  object.states.find(s => s.id === id)?.name ?? id;

/** A trigger as a title and a line: "Camera stop 2, Switch room" and what it does. */
export function triggerText(
  object: ObjectStates,
  trigger: StateTrigger,
  stops: readonly LayoutStop[]
): { title: string; detail: string } {
  const to = stateName(object, trigger.to);
  switch (trigger.kind) {
    case 'stop': {
      const at = stops.findIndex(s => s.id === trigger.stop);
      const name = at >= 0 ? `Camera stop ${at + 1}, ${stops[at]!.name}` : 'A camera stop';
      return { title: name, detail: `Goes to ${to} on arrival` };
    }
    case 'scroll':
      return {
        title: `Scroll ${n(trigger.start)}% to ${n(trigger.end)}% of the page`,
        detail: `Scrubs to ${to} with the scroll, back on the way up`,
      };
    case 'click':
      return {
        title: trigger.part ? `Click on ${trigger.part}` : 'Click on the model',
        detail: `Goes to ${to}`,
      };
    case 'hover':
      return {
        title: trigger.part ? `Hover on ${trigger.part}` : 'Hover on the model',
        detail: `Goes to ${to}, back when the pointer leaves`,
      };
  }
}

/** Where an eased move is at `t` (0 to 1). */
export function ease(easing: Easing, t: number): number {
  const x = Math.min(1, Math.max(0, t));
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

/** The pose between two at eased progress `t`. */
export function mixPose(a: PlainPose, b: PlainPose, t: number): PlainPose {
  const mix = (x: Vec3, y: Vec3): Vec3 => [
    x[0] + (y[0] - x[0]) * t,
    x[1] + (y[1] - x[1]) * t,
    x[2] + (y[2] - x[2]) * t,
  ];
  return { position: mix(a.position, b.position), rotation: mix(a.rotation, b.rotation) };
}

/**
 * Every part's pose partway through `t` from `from` to `to` (both state
 * ids), `progress` 0 to 1 over the move, staggered part by part.
 */
export function posesAt(
  object: ObjectStates,
  t: StateTransition,
  progress: number
): Record<string, PlainPose> {
  const a = object.states.find(s => s.id === t.from)?.parts ?? {};
  const b = object.states.find(s => s.id === t.to)?.parts ?? {};
  const parts = [...new Set([...Object.keys(a), ...Object.keys(b)])].sort();
  const duration = Math.max(n(t.duration), 1e-3);
  const stagger = n(t.stagger);
  const total = duration + stagger * Math.max(0, parts.length - 1);
  const out: Record<string, PlainPose> = {};
  parts.forEach((part, i) => {
    const local = (progress * total - stagger * i) / duration;
    out[part] = mixPose(poseOf(a[part]), poseOf(b[part]), ease(t.easing, local));
  });
  return out;
}

/** Every part's pose in a state. */
export function posesOf(object: ObjectStates, id: string): Record<string, PlainPose> {
  const parts = object.states.find(s => s.id === id)?.parts ?? {};
  return Object.fromEntries(Object.entries(parts).map(([p, pose]) => [p, poseOf(pose)]));
}
