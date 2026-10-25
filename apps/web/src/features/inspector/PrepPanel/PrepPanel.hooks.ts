import { useEffect } from 'react';
import type { InspectModel, ModelPrep } from '@/ipc/client';
import { previewKey } from '@/state/prepare';
import { assetKey, previewTarget, usePrepareStore } from '@/state/use-prepare-store';

/**
 * The model's asset's real size and fix-ups, saved on each change (the next
 * Build scene run applies them), with the preview on its newest version.
 */
export function usePrepPanel(project: string, model: InspectModel) {
  const asset = model.tweakAsset;
  const key = asset ? assetKey(project, asset) : '';
  const found = usePrepareStore(s => s.assets[key]) ?? null;
  const failed = usePrepareStore(s => s.failed[key]) ?? null;
  const load = usePrepareStore(s => s.load);
  const save = usePrepareStore(s => s.save);
  const run = usePrepareStore(s => s.preview);
  useEffect(() => {
    if (asset) void load(project, asset);
  }, [load, project, asset]);
  const target = { project, asset, inbox: null };
  const preview = usePrepareStore(s =>
    found ? s.previews[previewKey(previewTarget(target), found.entry)] : undefined
  );
  return {
    asset,
    found,
    failed,
    preview: preview ?? null,
    onChange: (next: ModelPrep) => {
      if (asset) void save(project, asset, next);
    },
    onPreview:
      found && found.version !== null ? () => void run({ ...target, plan: found.entry }) : null,
  };
}
