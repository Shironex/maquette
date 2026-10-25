/**
 * The price lists the core estimates sends with, for mock mode: Tripo's H
 * series table and Meshy's pricing page. Mirrors
 * crates/maquette-core/src/providers/pricing.rs.
 */
import type { MeshyPreset, PresetVariant, Provider, TripoPreset } from '../client';

export function tripoCredits(preset: TripoPreset, variant: PresetVariant): number {
  const legacy = preset.model.startsWith('v2');
  const pSeries = /^p/i.test(preset.model);
  let credits = 20;
  if (preset.texture) {
    credits += 10;
    if (!legacy) credits += { standard: 0, detailed: 10, extreme: 20 }[preset.textureQuality];
  }
  if (!legacy && !pSeries) {
    if (preset.geometryQuality === 'detailed') credits += 20;
    if (preset.smartLowPoly) credits += 10;
  }
  if (variant === 'character' && preset.rig) {
    credits += 25 + 10 * Math.min(preset.animations.length, 5);
  }
  return credits;
}

export function meshyCredits(preset: MeshyPreset, variant: PresetVariant): number {
  const cheap = preset.aiModel.startsWith('meshy-t') || preset.aiModel === 'meshy-6-lite';
  const k8 = preset.textureResolution === '8k';
  let credits = cheap
    ? preset.shouldTexture
      ? k8
        ? 20
        : 15
      : 5
    : preset.shouldTexture
      ? k8
        ? 35
        : 30
      : 20;
  if (variant === 'character' && preset.rig) credits += 5;
  return credits;
}

/** What a send should cost; null for By hand. */
export function estimate(
  provider: Provider,
  variant: PresetVariant,
  tripo: TripoPreset | null,
  meshy: MeshyPreset | null
): number | null {
  if (provider === 'tripo' && tripo) return tripoCredits(tripo, variant);
  if (provider === 'meshy' && meshy) return meshyCredits(meshy, variant);
  return null;
}
