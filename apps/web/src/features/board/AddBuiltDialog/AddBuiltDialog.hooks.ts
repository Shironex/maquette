import { useRef, useState, type FormEvent } from 'react';
import { commands, errorMessage, type AssetKind } from '@/ipc/client';
import { useTopEscape } from '@/lib/useTopEscape';
import { nameFromScript } from '@/state/built';
import { useBuiltStore } from '@/state/use-built-store';
import type { AddBuiltDialogProps } from './AddBuiltDialog.types';

/**
 * The script, its helpers, the name (from the script until typed) and the
 * add call; Escape closes while this is the top dialog.
 */
export function useAddBuiltDialog({ project, onClose }: AddBuiltDialogProps) {
  const add = useBuiltStore(s => s.add);
  const [script, setScript] = useState('');
  const [helpers, setHelpers] = useState<string[]>([]);
  const [name, setName] = useState('');
  const [named, setNamed] = useState(false);
  const [kind, setKind] = useState<AssetKind>('prop');
  const [root, setRoot] = useState('');
  const [replace, setReplace] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const ref = useRef<HTMLFormElement>(null);
  useTopEscape(ref, onClose);

  const changeScript = (path: string) => {
    setScript(path);
    if (!named) setName(nameFromScript(path));
    setError(null);
  };
  const pickScript = async () => {
    const picked = await commands.dialogPickScript('Pick the script that builds the model');
    if (picked) changeScript(picked);
  };
  const pickHelpers = async () => {
    const picked = await commands.dialogPickFiles('Pick the files the script imports or reads');
    setHelpers(h => [...h, ...picked.filter(p => !h.includes(p) && p !== script)]);
    setError(null);
  };
  const dropHelper = (path: string) => setHelpers(h => h.filter(p => p !== path));
  const rename = (next: string) => {
    setName(next);
    setNamed(true);
    setError(null);
  };

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!script) return setError('Pick the script that builds the model.');
    if (!name.trim()) return setError('Give the asset a name.');
    setSaving(true);
    try {
      await add(project, {
        name: name.trim(),
        kind,
        script,
        helpers,
        root: root.trim() || null,
        replace,
      });
      onClose();
    } catch (err) {
      setError(errorMessage(err));
      setSaving(false);
    }
  };

  return {
    ref,
    script,
    changeScript,
    pickScript,
    helpers,
    pickHelpers,
    dropHelper,
    name,
    rename,
    kind,
    setKind,
    root,
    setRoot,
    replace,
    setReplace,
    error,
    saving,
    submit,
  };
}

/** Whether Add built asset is open, and closing it, for the Board that mounts it. */
export function useBuiltDialog() {
  const open = useBuiltStore(s => s.adding);
  const setAdding = useBuiltStore(s => s.setAdding);
  return { open, close: () => setAdding(false) };
}
