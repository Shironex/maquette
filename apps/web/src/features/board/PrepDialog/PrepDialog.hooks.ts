import { useEffect, useRef, useState, type FormEvent } from 'react';
import type { ModelPrep } from '@/ipc/client';
import { useTopEscape } from '@/lib/useTopEscape';
import { previewKey, samePrep } from '@/state/prepare';
import { assetKey, previewTarget, usePrepareStore } from '@/state/use-prepare-store';
import type { PrepDialogProps } from './PrepDialog.types';

/**
 * The asset's saved real size and fix-ups, the copy being edited, its
 * preview on the newest version and Save. Escape closes while this is the
 * top dialog.
 */
export function usePrepDialog({ project, asset, onClose }: PrepDialogProps) {
  const ref = useRef<HTMLFormElement>(null);
  useTopEscape(ref, onClose);
  const key = assetKey(project, asset.id);
  const found = usePrepareStore(s => s.assets[key]) ?? null;
  const failed = usePrepareStore(s => s.failed[key]) ?? null;
  const saving = usePrepareStore(s => s.saving === key);
  const load = usePrepareStore(s => s.load);
  const save = usePrepareStore(s => s.save);
  const run = usePrepareStore(s => s.preview);
  const [draft, setDraft] = useState<ModelPrep | null>(null);
  useEffect(() => {
    void load(project, asset.id);
  }, [load, project, asset.id]);
  const value = draft ?? found?.entry ?? null;
  const target = { project, asset: asset.id, inbox: null };
  const preview = usePrepareStore(s =>
    value ? s.previews[previewKey(previewTarget(target), value)] : undefined
  );
  const changed = Boolean(value && found && !samePrep(value, found.entry));

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!value || saving) return;
    if (!changed || (await save(project, asset.id, value))) onClose();
  };
  return {
    ref,
    found,
    failed,
    value,
    setDraft,
    saving,
    changed,
    preview: preview ?? null,
    onPreview: found?.version != null && value ? () => void run({ ...target, plan: value }) : null,
    submit,
  };
}
