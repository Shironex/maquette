/**
 * When the Baked light card folds to one line: every model with a file skips
 * the bake (so the step bakes nothing), or the project ships to a
 * hand-built site. The card opens again on demand.
 */
import type { BakeSettings, BakeView, ShipStyle } from '@/ipc/client';

export function skippedText(count: number): string {
  if (count === 0) return 'Every model is baked.';
  if (count === 1) return '1 model skips the bake and keeps its own lighting.';
  return `${count} models skip the bake and keep their own lighting.`;
}

/** Why the card folds, as its one line says it, or null when it stays open. */
export function bakeFold(
  bake: Pick<BakeView, 'skipped'>,
  models: readonly string[],
  style: ShipStyle | null | undefined
): string | null {
  const skipped = new Set(bake.skipped);
  if (models.length > 0 && models.every(m => skipped.has(m))) {
    const who = models.length === 1 ? 'The only model skips' : `All ${models.length} models skip`;
    return `${who} the bake, so this step bakes nothing.`;
  }
  if (style === 'models') {
    return `Ships to a hand-built site. ${skippedText(bake.skipped.length)}`;
  }
  return null;
}

/** "1024 px · 32 samples · denoise", the settings in one line. */
export function settingsLine(settings: BakeSettings): string {
  return `${settings.size} px · ${settings.samples} samples · ${settings.denoise ? 'denoise' : 'no denoise'}`;
}
