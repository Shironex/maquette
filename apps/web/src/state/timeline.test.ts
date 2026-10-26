import { describe, expect, it } from 'vitest';
import type { LayoutStop, ObjectStates, TimelineView } from '@/ipc/client';
import {
  addExplode,
  addStopMark,
  amountLabel,
  asKind,
  draftOf,
  explodeMarks,
  nextState,
  percentToStop,
  removeTrigger,
  sameDraft,
  scrollRange,
  setExplode,
  setExplodeKey,
  stateAt,
  stateMarks,
  stopPercent,
} from './timeline';
import { placeLine, siteSummary, timelineChecks, unitLabel } from './timeline-checks';

const stop = (id: string): LayoutStop => ({
  id,
  name: id[0]!.toUpperCase() + id.slice(1),
  position: [0, 1, 2],
  target: [0, 0, 0],
  fov: 35,
  models: null,
});

const STOPS = ['porch', 'sakura', 'shrubs', 'canopy', 'pullback'].map(stop);

const cabinet = (): ObjectStates => ({
  states: [
    { id: 'closed', name: 'Closed', parts: {} },
    { id: 'open', name: 'Door open', parts: {} },
    { id: 'on', name: 'Breakers on', parts: {} },
  ],
  transitions: [],
  triggers: [
    { kind: 'stop', to: 'open', stop: 'sakura', part: null, start: null, end: null },
    { kind: 'scroll', to: 'on', stop: null, part: null, start: 55, end: 70 },
    { kind: 'click', to: 'on', stop: null, part: 'Breaker 1', start: null, end: null },
  ],
});

const view = (): TimelineView => ({
  project: 'moon-island',
  stops: STOPS,
  models: [{ asset: 'cabinet', name: 'Switch cabinet', thumbnail: null, parts: ['Body', 'Door'] }],
  states: { cabinet: cabinet() },
  timeline: { explode: {} },
});

describe('the timeline rules', () => {
  it('spreads the stops evenly over the page and back', () => {
    expect(STOPS.map((_, i) => stopPercent(i, STOPS.length))).toEqual([0, 25, 50, 75, 100]);
    expect(percentToStop(62.5, 5)).toBe(2.5);
    expect(stopPercent(0, 1)).toBe(0);
    expect(percentToStop(40, 1)).toBe(0);
  });

  it('keeps only the scroll driven triggers in the draft', () => {
    const draft = draftOf(view());
    expect(draft.triggers.cabinet?.map(t => t.kind)).toEqual(['stop', 'scroll']);
    expect(sameDraft(draft, draftOf(view()))).toBe(true);
  });

  it('places the marks and knows the state at any point', () => {
    const draft = draftOf(view());
    const marks = stateMarks(cabinet(), draft.triggers.cabinet!, STOPS);
    expect(marks.map(m => [m.kind, m.start, m.end, m.label])).toEqual([
      ['stop', 25, 25, 'Door open'],
      ['scroll', 55, 70, 'Breakers on'],
    ]);
    const at = (p: number) => stateAt(cabinet(), draft.triggers.cabinet!, STOPS, p);
    expect([at(10), at(30), at(60), at(80)]).toEqual(['closed', 'open', 'open', 'on']);
    expect(nextState(cabinet(), draft.triggers.cabinet!, STOPS, 90)).toBe('closed');
  });

  it('adds a stop mark going on to the next state', () => {
    const draft = draftOf(view());
    const added = addStopMark(draft, 'cabinet', cabinet(), STOPS, 'shrubs');
    expect(added?.index).toBe(2);
    expect(added?.draft.triggers.cabinet?.[2]).toMatchObject({ stop: 'shrubs', to: 'on' });
    const single = { ...cabinet(), states: cabinet().states.slice(0, 1) };
    expect(addStopMark(draft, 'cabinet', single, STOPS, 'shrubs')).toBeNull();
    expect(removeTrigger(added!.draft, 'cabinet', 2).triggers.cabinet).toHaveLength(2);
  });

  it('turns a stop into the range before it and a range into its nearest stop', () => {
    const [atStop, range] = draftOf(view()).triggers.cabinet!;
    expect(asKind(atStop!, 'scroll', STOPS)).toMatchObject({ kind: 'scroll', start: 0, end: 25 });
    expect(asKind(range!, 'stop', STOPS)).toMatchObject({ kind: 'stop', stop: 'canopy' });
    expect(scrollRange(80, 20)).toEqual({ start: 80, end: 81 });
    expect(scrollRange(-5, 140)).toEqual({ start: 0, end: 100 });
  });

  it('gives a new explode track two keys and eases them in stop order', () => {
    let draft = addExplode(draftOf(view()), 'cabinet', STOPS);
    const track = draft.timeline.explode.cabinet!;
    expect(track.keys).toEqual([
      { stop: 'porch', amount: 0 },
      { stop: 'pullback', amount: 1 },
    ]);
    draft = setExplodeKey(draft, 'cabinet', 'shrubs', 1);
    draft = setExplode(draft, 'cabinet', { distance: 40 });
    expect(draft.timeline.explode.cabinet?.distance).toBe(10);
    const { keys, spans } = explodeMarks(draft.timeline.explode.cabinet!, STOPS);
    expect(keys.map(k => k.stop)).toEqual(['porch', 'shrubs', 'pullback']);
    expect(spans).toEqual([{ from: 0, to: 50, label: 'Spreads apart' }]);
    expect(
      setExplodeKey(draft, 'cabinet', 'shrubs', null).timeline.explode.cabinet?.keys
    ).toHaveLength(2);
    expect([amountLabel(0), amountLabel(0.4), amountLabel(1)]).toEqual([
      'Together',
      '40% apart',
      'Apart',
    ]);
  });
});

describe('the timeline checks', () => {
  it('warns about a change that changes nothing, overlapping ranges and a still track', () => {
    const draft = draftOf(view());
    draft.triggers.cabinet!.push(
      { kind: 'stop', to: 'open', stop: 'shrubs', part: null, start: null, end: null },
      { kind: 'scroll', to: 'closed', stop: null, part: null, start: 65, end: 90 }
    );
    draft.timeline.explode.cabinet = { distance: 0.2, easing: 'linear', keys: [] };
    const titles = timelineChecks(view(), draft).map(c => c.title);
    expect(titles).toEqual([
      'No change at Shrubs',
      'Two ranges of Switch cabinet overlap',
      'Switch cabinet never spreads',
    ]);
    expect(siteSummary(view(), draft)).toEqual({ lanes: 2, marks: 4 });
  });

  it('says where the playhead is and counts in the chosen units', () => {
    expect(placeLine(1, STOPS)).toBe('at Sakura');
    expect(placeLine(1.4, STOPS)).toBe('between Sakura and Shrubs');
    expect(unitLabel(46, 'percent', 5, null)).toBe('46%');
    expect(unitLabel(50, 'pixels', 5, 6000)).toBe('3,000 px');
    expect(unitLabel(50, 'stops', 5, null)).toBe('stop 3');
  });
});
