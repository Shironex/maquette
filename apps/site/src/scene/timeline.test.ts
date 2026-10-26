import { describe, expect, it } from 'vitest';
import {
  ease,
  explodeAt,
  percentOf,
  settleAt,
  stateAnimator,
  stateEvents,
  type ObjectStates,
} from './timeline';

const STOPS = ['porch', 'sakura', 'shrubs', 'canopy', 'pullback'];

const cabinet = (): ObjectStates => ({
  states: [
    { id: 'closed', name: 'Closed', parts: {} },
    {
      id: 'open',
      name: 'Door open',
      parts: { Door: { position: [0, 0, 0], rotation: [0, -100, 0] } },
    },
    {
      id: 'on',
      name: 'Breakers on',
      parts: {
        Door: { position: [0, 0, 0], rotation: [0, -100, 0] },
        Breaker: { position: [0, 0.02, 0], rotation: [-30, 0, 0] },
      },
    },
  ],
  transitions: [
    { from: 'closed', to: 'open', duration: 1, delay: 0, easing: 'linear', stagger: 0 },
    { from: 'open', to: 'on', duration: 0.4, delay: 0, easing: 'linear', stagger: 0 },
  ],
  triggers: [
    { kind: 'stop', to: 'open', stop: 'sakura', part: null, start: null, end: null },
    { kind: 'scroll', to: 'on', stop: null, part: null, start: 55, end: 70 },
    { kind: 'click', to: 'closed', stop: null, part: 'Door', start: null, end: null },
  ],
});

const doorY = (pose: ReturnType<typeof settleAt>['pose']) => pose.Door?.rotation[1] ?? 0;

describe('the scroll timeline on the site', () => {
  it('counts the page in camera stops', () => {
    expect(percentOf(1, 5)).toBe(25);
    expect(percentOf(9, 5)).toBe(100);
    expect(percentOf(3, 1)).toBe(0);
  });

  it('places the stop and scroll changes and leaves clicks to the page', () => {
    expect(stateEvents(cabinet(), STOPS).map(e => [e.kind, e.start, e.end])).toEqual([
      ['stop', 25, 25],
      ['scroll', 55, 70],
    ]);
    expect(stateEvents(cabinet(), ['porch'])).toHaveLength(1);
  });

  it('settles on the changes the scroll has passed and scrubs a range under way', () => {
    const object = cabinet();
    const events = stateEvents(object, STOPS);
    expect(settleAt(object, events, 10, 0).state).toBe('closed');
    expect(doorY(settleAt(object, events, 30, 0).pose)).toBe(-100);
    const half = settleAt(object, events, 62.5, 0).pose;
    expect(half.Breaker?.rotation[0]).toBeCloseTo(-15);
    expect(settleAt(object, events, 80, 0).state).toBe('on');
  });

  it('plays a stop change over its transition and back when the page goes up', () => {
    const play = stateAnimator(cabinet(), STOPS);
    expect(play.update(0, 0).moving).toBe(false);
    const started = play.update(1, 10);
    expect(started.moving).toBe(true);
    expect(doorY(started.pose)).toBe(0);
    expect(doorY(play.update(1, 10.5).pose)).toBeCloseTo(-50);
    expect(play.update(1, 11.01)).toMatchObject({ moving: false });
    expect(doorY(play.update(1, 11.02).pose)).toBe(-100);
    const back = play.update(0.2, 20);
    expect(back.moving).toBe(true);
    expect(doorY(play.update(0.2, 21).pose)).toBeCloseTo(0);
  });

  it('opens on the settled pose without playing what came before', () => {
    const play = stateAnimator(cabinet(), STOPS);
    const first = play.update(3, 0);
    expect(first.moving).toBe(false);
    expect(first.pose.Breaker?.rotation[0]).toBe(-30);
  });

  it('eases an explode track between its keyed stops and holds it outside them', () => {
    const track = {
      distance: 0.3,
      easing: 'linear' as const,
      keys: [
        { stop: 'pullback', amount: 1 },
        { stop: 'canopy', amount: 0 },
        { stop: 'gone', amount: 0.5 },
      ],
    };
    expect(explodeAt(track, STOPS, 1)).toBe(0);
    expect(explodeAt(track, STOPS, 3.5)).toBeCloseTo(0.5);
    expect(explodeAt(track, STOPS, 4)).toBe(1);
    expect(explodeAt({ ...track, keys: [] }, STOPS, 2)).toBe(0);
  });

  it('eases as Maquette does', () => {
    expect(ease('ease-in-out', 0.5)).toBeCloseTo(0.5);
    expect(ease('ease-out', 0.5)).toBeCloseTo(0.875);
    expect(ease('linear', 2)).toBe(1);
  });
});
