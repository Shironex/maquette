import { useState, type FormEvent } from 'react';
import { commands, errorMessage, type AssetKind, type Stage } from '@/ipc/client';
import type { AddAssetFormProps } from './AddAssetForm.types';

/** The form's fields, its error and the add call on submit. */
export function useAddAssetForm({
  project,
  onDone,
}: Pick<AddAssetFormProps, 'project' | 'onDone'>) {
  const [name, setName] = useState('');
  const [kind, setKind] = useState<AssetKind>('prop');
  const [stage, setStage] = useState<Stage>('concept');
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!name.trim()) {
      setError('Give the asset a name.');
      return;
    }
    setSaving(true);
    try {
      onDone(await commands.assetAdd(project, { name: name.trim(), kind, stage }));
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
  return { name, rename, kind, setKind, stage, setStage, error, saving, submit };
}
