/**
 * The Paths editor in mock mode: Moon Island's paths kept in the fake
 * layout, saved whole with their energize tracks as the app saves them.
 *
 * `?paths=` in the URL (`?timeline=demo` brings `demo` along):
 * - `demo`: a lantern cable from the shop house to the vending machine
 *   that lights up between Sakura and Canopy, a root glow up the sakura
 *   that is always lit, and a draft path with no track.
 * - `empty`: no paths yet.
 */
import type { PathTrack, ScenePath } from '../client';
import { fail, type Args } from './backend-options';
import type { createFakeLayout } from './layout';

type FakeLayout = ReturnType<typeof createFakeLayout>;

const look = { width: 0.06, color: '#f2c230', glow: '#ffe08a' };

export const DEMO_PATHS: ScenePath[] = [
  {
    id: 'lantern-cable',
    name: 'Lantern cable',
    curve: 'catmull-rom',
    points: [
      [-3.2, 0, 3.4],
      [-2.2, 0.05, 4.8],
      [0, 0.05, 5.2],
      [2.4, 0.05, 4.2],
      [3.6, 0, 1.2],
    ],
    start: 'shop-house',
    end: 'vending',
    ...look,
  },
  {
    id: 'root-glow',
    name: 'Root glow',
    curve: 'polyline',
    points: [
      [0.6, 0, 1.4],
      [0.5, 1.8, 0.9],
      [0.3, 3.6, 0.6],
    ],
    start: null,
    end: null,
    width: 0.08,
    color: '#5f7a5c',
    glow: '#d6f5c9',
  },
  {
    id: 'spare-run',
    name: 'Spare run',
    curve: 'polyline',
    points: [
      [-2.5, 0, -2],
      [2.5, 0, -2.6],
    ],
    start: null,
    end: null,
    width: 0.04,
    color: '#9a9488',
    glow: '#e9e5dc',
  },
];

export const DEMO_TRACKS: Record<string, PathTrack> = {
  'lantern-cable': {
    easing: 'linear',
    keys: [
      { stop: 'sakura', amount: 0 },
      { stop: 'canopy', amount: 1 },
    ],
  },
  'root-glow': { easing: 'linear', keys: [{ stop: 'porch', amount: 1 }] },
};

export function createPathsHandlers(deps: { layout: FakeLayout; paths: string | null }) {
  if (deps.paths === 'demo') deps.layout.setPaths('moon-island', DEMO_PATHS, DEMO_TRACKS);
  if (deps.paths === 'empty') deps.layout.setPaths('moon-island', [], {});

  return {
    paths_save: ({ project, paths, tracks }: Args) => {
      const id = String(project);
      const list = paths as ScenePath[];
      const known = new Set(deps.layout.view(id).models.map(m => m.asset));
      for (const path of list) {
        if (!path.name.trim()) fail('invalid', 'Give every path a name.');
        for (const anchor of [path.start, path.end]) {
          if (anchor && !known.has(anchor)) {
            fail('invalid', `The path ${path.name} is anchored to ${anchor}, which has no model.`);
          }
        }
      }
      deps.layout.setPaths(id, list, tracks as Record<string, PathTrack>);
      return deps.layout.view(id);
    },
  };
}
