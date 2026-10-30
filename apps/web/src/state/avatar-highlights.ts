/**
 * The highlights a generated model's lighting painted into its base color
 * texture: what an MToon export does with them, and what the last one did.
 */
import type { Highlights, HighlightsReport } from '@/ipc/client';
import type { Choice } from './avatar';

export const HIGHLIGHTS: readonly (Choice<Highlights> & { hint: string })[] = [
  {
    value: 'soften',
    label: 'Soften',
    hint: "Tones down only strong highlights baked into dark material, toward the colour around them, so they do not read as white blotches. A copy of the texture is softened; the model's own stays as it is.",
  },
  { value: 'keep', label: 'Keep', hint: 'Writes the texture as it is.' },
];

/** "3.0% softened", "none found" or "kept as it is", with why when nothing changed. */
export function highlightsSummary(report: HighlightsReport): string {
  if (report.mode === 'keep') return 'Kept as they are';
  const softened = report.images.filter(i => i.softened);
  if (softened.length === 0) return report.note ?? 'None found';
  const share = Math.max(...softened.map(i => i.coverage ?? 0));
  return `${(share * 100).toFixed(1)}% softened`;
}
