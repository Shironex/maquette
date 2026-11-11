/**
 * Mock starter scripts: what a project made today has, and what an older one
 * shows on the Pipeline view's banner.
 */
import type { StarterState, StarterStatus } from '../client';

/** The helper file for built asset scripts and its example: every project gets them. */
export const BUILT_STARTERS = ['built/maquette_helpers.py', 'built/maquette_example.py'] as const;

/** The diorama starters, as a project made today has them. */
export const DIORAMA_STARTERS = [
  'tools/build_scene.py',
  'tools/bake_light.py',
  'tools/export_web.py',
  'tools/render_stops.py',
  'tools/ship.mjs',
  ...BUILT_STARTERS,
] as const;

export function currentStarters(): StarterStatus[] {
  return DIORAMA_STARTERS.map(path => ({ path, state: 'current', backup: null }));
}

/**
 * Frostveil was made before the starters grew up: three untouched old
 * copies, one the user edited, and no helper file for built scripts yet.
 */
export function oldStarters(): StarterStatus[] {
  const state: Record<string, StarterState> = {
    'tools/build_scene.py': 'outdated',
    'tools/bake_light.py': 'outdated',
    'tools/export_web.py': 'edited',
    'tools/render_stops.py': 'outdated',
    'built/maquette_helpers.py': 'missing',
    'built/maquette_example.py': 'missing',
  };
  return DIORAMA_STARTERS.map(path => ({ path, state: state[path] ?? 'current', backup: null }));
}
