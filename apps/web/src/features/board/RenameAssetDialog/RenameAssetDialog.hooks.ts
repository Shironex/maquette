import { useRef, useState, type FormEvent } from 'react';
import { commands, errorMessage } from '@/ipc/client';
import { useTopEscape } from '@/lib/useTopEscape';
import type { RenameAssetDialogProps } from './RenameAssetDialog.types';

/** The longest asset name; the core says the same. */
export const MAX_NAME_CHARS = 80;

/** The new name, its checks and the rename call; Escape closes while this is the top dialog. */
export function useRenameAssetDialog({ project, asset, onDone, onClose }: RenameAssetDialogProps) {
  const [name, setName] = useState(asset.name);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const ref = useRef<HTMLFormElement>(null);
  useTopEscape(ref, onClose);
  const trimmed = name.trim();
  const tooLong = trimmed.length > MAX_NAME_CHARS;
  const unchanged = trimmed === asset.name;

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!trimmed || tooLong || saving) return;
    if (unchanged) {
      onClose();
      return;
    }
    setSaving(true);
    try {
      onDone(await commands.assetRename(project, asset.id, trimmed));
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setSaving(false);
    }
  };
  const rename = (next: string) => {
    setName(next);
    setError(null);
  };

  return { ref, name, rename, trimmed, tooLong, error, saving, submit };
}
