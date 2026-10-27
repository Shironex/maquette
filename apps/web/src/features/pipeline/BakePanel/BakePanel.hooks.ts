import { useEffect, useState } from 'react';
import type { BakePatch, BakeSettings, BakeView } from '@/ipc/client';

/** What the default bake costs: a lightmap of this edge at these samples. */
const BASE = { size: 1024, samples: 32 };

/**
 * Roughly how long a bake takes next to the default one. Cycles time grows
 * with the texels and the samples per texel, so 2048 px costs about four
 * times 1024 px and 64 samples about twice 32.
 */
export function relativeCost(settings: BakeSettings): number {
  return (settings.size / BASE.size) ** 2 * (settings.samples / BASE.samples);
}

export function costText(settings: BakeSettings): string {
  const cost = relativeCost(settings);
  if (Math.abs(cost - 1) < 0.05) return 'The default bake time';
  const shown = cost >= 10 ? Math.round(cost) : Math.round(cost * 10) / 10;
  return `About ${shown}x the default bake time`;
}

/** The fields that differ from what is saved. */
export function bakePatch(saved: BakeSettings, draft: BakeSettings): BakePatch {
  return {
    size: draft.size === saved.size ? null : draft.size,
    samples: draft.samples === saved.samples ? null : draft.samples,
    denoise: draft.denoise === saved.denoise ? null : draft.denoise,
  };
}

/** The settings being edited, the saved ones they start from, and Apply. */
export function useBakePanel(bake: BakeView, onSave: (patch: BakePatch) => Promise<boolean>) {
  const saved = bake.settings;
  const [draft, setDraft] = useState<BakeSettings>(saved);
  // A save, an agent's change or another project brings the saved values in.
  const { size, samples, denoise } = saved;
  useEffect(() => setDraft({ size, samples, denoise }), [size, samples, denoise]);
  const [saving, setSaving] = useState(false);
  const patch = bakePatch(saved, draft);
  const dirty = Object.values(patch).some(v => v !== null);
  const update = (patch: Partial<BakeSettings>) => setDraft(now => ({ ...now, ...patch }));
  const save = async () => {
    setSaving(true);
    await onSave(patch);
    setSaving(false);
  };
  return { draft, dirty, update, save, saving, reset: () => setDraft(saved) };
}

/** Whether a card that folds shows its settings, opened on demand; edits keep it open. */
export function useBakeFold(fold: string | null, dirty: boolean) {
  const [expanded, setExpanded] = useState(false);
  return {
    folded: fold !== null && !expanded && !dirty,
    canFold: fold !== null,
    expand: () => setExpanded(true),
    collapse: () => setExpanded(false),
  };
}
