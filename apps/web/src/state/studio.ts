/**
 * By hand on Tripo: the Tripo Studio tab and rig a send asks for. The
 * defaults follow `handoff/options.rs`, so the panel shows what the card
 * will say before it is written.
 */
import type {
  ModelOption,
  PresetVariant,
  StudioMesh,
  StudioRig,
  StudioSettings,
  TripoPreset,
} from '@/ipc/client';
import { formatCount } from '@/lib/format';
import { isTripoPSeries, modelLabel } from './provider-presets';

export const STUDIO_MESH: { value: StudioMesh; label: string }[] = [
  { value: 'hd-model', label: 'HD Model' },
  { value: 'smart-mesh', label: 'Smart Mesh (P2.0)' },
];

export const STUDIO_RIG: { value: StudioRig; label: string }[] = [
  { value: 'biped', label: 'Biped (Mixamo)' },
  { value: 'quadruped', label: 'Quadruped' },
  { value: 'multi-legged', label: 'Multi-legged' },
  { value: 'none', label: 'None' },
];

const label = <T extends string>(options: { value: T; label: string }[], value: T) =>
  options.find(o => o.value === value)?.label ?? value;

/** What the preset says: Smart Mesh for a P series or smart low poly one, biped for a rigged character. */
export function studioDefaults(preset: TripoPreset, variant: PresetVariant): StudioSettings {
  return {
    mesh: isTripoPSeries(preset.model) || preset.smartLowPoly ? 'smart-mesh' : 'hd-model',
    rig: variant === 'character' && preset.rig ? 'biped' : 'none',
  };
}

/** "Multiview to 3D with 3 views" or "Image to 3D": the card's mode follows the images. */
export function studioMode(views: number): string {
  return views >= 2 ? `Multiview to 3D with ${views} views` : 'Image to 3D';
}

const RIG_WORDS: Record<StudioRig, string> = {
  biped: 'biped rig (Mixamo)',
  quadruped: 'quadruped rig',
  'multi-legged': 'multi-legged rig',
  none: 'no rig',
};

const rigWords = (rig: StudioRig) => RIG_WORDS[rig];

/** "Smart Mesh (P2.0), quadruped rig": what an asset changes from the batch. */
export function studioChanges(changes: Partial<StudioSettings>): string[] {
  return [
    changes.mesh ? label(STUDIO_MESH, changes.mesh) : null,
    changes.rig ? rigWords(changes.rig) : null,
  ].filter((p): p is string => Boolean(p));
}

/** "Tripo Studio HD Model, H3.1, 50,000 faces, PBR, biped rig (Mixamo)": a By hand Tripo send in a line. */
export function studioSummary(
  preset: TripoPreset,
  studio: StudioSettings,
  models: readonly ModelOption[]
): string {
  const hd = studio.mesh === 'hd-model';
  return [
    `Tripo Studio ${label(STUDIO_MESH, studio.mesh)}`,
    hd && !isTripoPSeries(preset.model) ? modelLabel(models, preset.model) : null,
    preset.faceLimit ? `${formatCount(preset.faceLimit)} faces` : null,
    preset.pbr ? 'PBR' : null,
    rigWords(studio.rig),
  ]
    .filter(Boolean)
    .join(', ');
}
