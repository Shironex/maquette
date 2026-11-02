import type { PlanPreset } from '@/ipc/client';

/** A preset as the New canvas dialog offers it. */
export interface PresetChoice {
  id: PlanPreset;
  name: string;
  /** What it starts with, in one line. */
  blurb: string;
  /** A small tag beside the name. */
  tag?: string;
}

/**
 * The presets, in the order the dialog shows them. They only set what a
 * canvas starts with (the core's `plans/presets.rs` lays out the starter
 * notes); every canvas holds every kind of node. The camera plan is built
 * in, never a preset.
 */
export const PRESETS: readonly PresetChoice[] = [
  {
    id: 'brainstorm',
    name: 'Brainstorm',
    tag: 'Default',
    blurb: 'Blank. Notes, pictures, links and assets anywhere.',
  },
  {
    id: 'story',
    name: 'Story',
    blurb: 'Beats left to right, characters above, places below. Beats can branch.',
  },
  {
    id: 'mechanics',
    name: 'Game mechanics',
    tag: 'For games',
    blurb: 'One card per system with its verb, look and input.',
  },
];

/** The preset's name as the list, the dialog and the canvas show it. */
export const PRESET_NAME: Record<PlanPreset, string> = {
  brainstorm: 'Brainstorm',
  story: 'Story',
  mechanics: 'Game mechanics',
};
