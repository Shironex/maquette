/**
 * The Timeline view in mock mode: the fake layout's camera stops, every
 * model with the parts the Rust reader reported for its bundled file, the
 * object states and the explode tracks.
 *
 * `?timeline=` in the URL (it brings the `?motion=demo` models along):
 * - `demo`: the switch cabinet opens its door at Sakura, turns its
 *   breakers on from 55 to 70 percent and spreads apart into Pullback.
 * - `empty`: like demo, but Moon Island has no camera stops yet.
 */
import type { ObjectStates, StateTrigger, Timeline, TimelineView } from '../client';
import { fail, type Args } from './backend-options';
import { cabinetStates, motionFiles } from './create-motion-handlers';
import type { createFakeLayout } from './layout';
import { LAYOUT_FILES } from './layout';
import { MOTION_FIXTURES } from './motion-fixtures';

type FakeLayout = ReturnType<typeof createFakeLayout>;

/** The `?timeline=demo` cabinet: its triggers moved to where the tour reaches it. */
export function demoCabinet(): ObjectStates {
  const states = cabinetStates();
  return {
    ...states,
    triggers: [
      { kind: 'stop', to: 'door-open', stop: 'sakura', part: null, start: null, end: null },
      { kind: 'scroll', to: 'breakers-on', stop: null, part: null, start: 55, end: 70 },
      { kind: 'click', to: 'breakers-on', stop: null, part: 'Breaker 1', start: null, end: null },
    ],
  };
}

const DEMO_TIMELINE: Timeline = {
  explode: {
    'switch-cabinet': {
      distance: 0.3,
      easing: 'ease-in-out',
      keys: [
        { stop: 'canopy', amount: 0 },
        { stop: 'pullback', amount: 1 },
      ],
    },
  },
  paths: {},
};

export function createTimelineHandlers(deps: { layout: FakeLayout; timeline: string | null }) {
  const files = { ...LAYOUT_FILES, ...motionFiles(deps.timeline ? 'demo' : null) };
  if (deps.timeline) {
    deps.layout.setMotion('moon-island', { 'switch-cabinet': demoCabinet() }, {});
    deps.layout.setTimeline('moon-island', {}, DEMO_TIMELINE);
  }
  if (deps.timeline === 'empty') deps.layout.clearStops('moon-island');

  function view(id: string): TimelineView {
    const { layout, models } = deps.layout.view(id);
    return {
      project: id,
      stops: layout.stops,
      models: models.map(m => ({
        asset: m.asset,
        name: m.name,
        thumbnail: null,
        parts: MOTION_FIXTURES[files[m.path] ?? '']?.parts ?? [m.name],
      })),
      states: structuredClone(layout.states),
      paths: structuredClone(layout.paths),
      timeline: structuredClone(layout.timeline),
    };
  }

  return {
    timeline_get: ({ project }: Args) => view(String(project)),
    timeline_save: ({ project, triggers, timeline }: Args) => {
      const id = String(project);
      const known = new Set(deps.layout.view(id).models.map(m => m.asset));
      const tracks = (timeline as Timeline).explode;
      for (const asset of Object.keys(tracks)) {
        if (!known.has(asset)) fail('invalid', `There is no model for the asset ${asset}.`);
      }
      deps.layout.setTimeline(id, triggers as Record<string, StateTrigger[]>, timeline as Timeline);
      return view(id);
    },
  };
}
