import { useEffect, useState } from 'react';
import type { GameFormat, GameSettings, GameSettingsChange, GameTextures } from '@/ipc/client';

export interface GameExportDraft {
  format: GameFormat;
  textures: GameTextures;
  textureCap: number;
  bake: boolean;
}

export const TEXTURE_CAPS = [256, 512, 1024, 2048, 4096] as const;

export function draftOf(settings: GameSettings): GameExportDraft {
  return {
    format: settings.format,
    textures: settings.export.textures,
    textureCap: settings.export.textureCap,
    bake: settings.bake,
  };
}

/**
 * Which cap each folder's models export at: a folder with a budget keeps its
 * own texture size, the cap is for the rest.
 */
export function capLine(settings: GameSettings): string {
  const rows = settings.budgets.map(
    b => `${b.folder ? `${b.folder}/` : 'top level'} ${b.texture} px`
  );
  if (!rows.length) return 'Every model exports at this cap: no folder has a budget.';
  return `Folders with a budget export at its texture size: ${rows.join(', ')}. This cap is for models in folders without one.`;
}

/** The change Apply saves: only what differs from the saved settings. */
export function exportChange(saved: GameSettings, draft: GameExportDraft): GameSettingsChange {
  const exportChanged =
    draft.textures !== saved.export.textures || draft.textureCap !== saved.export.textureCap;
  return {
    format: draft.format === saved.format ? null : draft.format,
    export: exportChanged ? { textures: draft.textures, textureCap: draft.textureCap } : null,
    bake: draft.bake === saved.bake ? null : draft.bake,
  };
}

/** The export settings being edited, the saved ones they start from, and Apply. */
export function useGameExportPanel(
  settings: GameSettings,
  onSave: (change: GameSettingsChange) => Promise<boolean>
) {
  const saved = draftOf(settings);
  const [draft, setDraft] = useState(saved);
  const { format, textures, textureCap, bake } = saved;
  // A save, an agent's change or another project brings the saved values in.
  useEffect(
    () => setDraft({ format, textures, textureCap, bake }),
    [format, textures, textureCap, bake]
  );
  const [saving, setSaving] = useState(false);
  const change = exportChange(settings, draft);
  const dirty = Object.values(change).some(v => v !== null && v !== undefined);
  const update = (patch: Partial<GameExportDraft>) => setDraft(now => ({ ...now, ...patch }));
  const save = async () => {
    setSaving(true);
    await onSave(change);
    setSaving(false);
  };
  return { draft, dirty, update, save, saving, reset: () => setDraft(saved) };
}
