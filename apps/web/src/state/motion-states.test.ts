import { describe, expect, it } from 'vitest';
import type { ObjectStates } from '@/ipc/client';
import {
  addState,
  addTrigger,
  ease,
  firstStates,
  newTrigger,
  partsInMove,
  posesAt,
  removeState,
  setPose,
  stateId,
  transitionInto,
  triggerText,
} from './motion-states';

const door = {
  position: [0, 0, 0] as [number, number, number],
  rotation: [0, -90, 0] as [number, number, number],
};

function cabinet(): ObjectStates {
  let o = addState(firstStates(), 'Door open');
  o = setPose(o, 'door-open', 'Door', door);
  return o;
}

describe('object states', () => {
  it('makes unique ids from names', () => {
    expect(stateId('Door open', [])).toBe('door-open');
    expect(stateId('Door open', ['door-open'])).toBe('door-open-2');
    expect(stateId('!!', [])).toBe('state');
  });

  it('adds a state posed like the last, with a move into it', () => {
    const o = addState(cabinet(), 'Wide open');
    expect(o.states.map(s => s.id)).toEqual(['rest', 'door-open', 'wide-open']);
    expect(o.states[2]!.parts.Door).toEqual(door);
    expect(transitionInto(o, 'wide-open')?.transition.from).toBe('door-open');
  });

  it('drops a pose at rest and never removes the rest state', () => {
    const o = setPose(cabinet(), 'door-open', 'Door', { position: [0, 0, 0], rotation: [0, 0, 0] });
    expect(o.states[1]!.parts).toEqual({});
    expect(removeState(o, 'rest')).toBe(o);
    const gone = removeState(
      addTrigger(cabinet(), newTrigger('click', 'door-open', [])),
      'door-open'
    );
    expect(gone).toEqual({ states: [firstStates().states[0]], transitions: [], triggers: [] });
  });

  it('moves the parts posed differently, eased and staggered', () => {
    const o = cabinet();
    const t = o.transitions[0]!;
    expect(partsInMove(o, t)).toEqual(['Door']);
    expect(posesAt(o, t, 0).Door!.rotation[1]).toBeCloseTo(0);
    expect(posesAt(o, { ...t, easing: 'linear' }, 0.5).Door!.rotation[1]).toBeCloseTo(-45);
    expect(posesAt(o, t, 1).Door!.rotation[1]).toBeCloseTo(-90);
    expect(ease('ease-in-out', 0.5)).toBeCloseTo(0.5);
    expect(ease('ease-out', 2)).toBe(1);
  });

  it('describes triggers', () => {
    const stops = [
      {
        id: 'arrival',
        name: 'Arrival',
        position: [0, 0, 0],
        target: [0, 0, 0],
        fov: 36,
        models: null,
      },
    ] as unknown as Parameters<typeof triggerText>[2];
    const o = cabinet();
    expect(triggerText(o, newTrigger('stop', 'door-open', stops), stops)).toEqual({
      title: 'Camera stop 1, Arrival',
      detail: 'Goes to Door open on arrival',
    });
    expect(triggerText(o, newTrigger('scroll', 'door-open', stops), stops).title).toBe(
      'Scroll 40% to 55% of the page'
    );
  });
});
