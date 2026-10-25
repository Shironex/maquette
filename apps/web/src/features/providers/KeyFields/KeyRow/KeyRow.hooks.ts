import { useState } from 'react';
import type { KeyTest, ProviderId } from '@/ipc/client';
import { useProvidersStore } from '@/state/use-providers-store';

/** One service's saved key, the draft for a new one and the last test result. */
export function useKeyRow(service: ProviderId) {
  const status = useProvidersStore(s => s.keys.find(k => k.service === service));
  const busy = useProvidersStore(s => s.keyBusy === service);
  const save = useProvidersStore(s => s.saveKey);
  const remove = useProvidersStore(s => s.removeKey);
  const test = useProvidersStore(s => s.testKey);
  const [draft, setDraft] = useState('');
  const [editing, setEditing] = useState(false);
  const [result, setResult] = useState<KeyTest | null>(null);
  const saved = Boolean(status?.saved);
  const showInput = !saved || editing;

  const onSave = async () => {
    if (!(await save(service, draft))) return;
    setDraft('');
    setEditing(false);
    setResult(null);
  };
  const onTest = async () => setResult(await test(service, showInput && draft ? draft : null));
  // A test result speaks for the key it tested, so it goes with that key.
  const onRemove = async () => {
    setResult(null);
    await remove(service);
  };
  const replace = (on: boolean) => {
    setEditing(on);
    setDraft('');
    setResult(null);
  };

  return {
    status,
    busy,
    draft,
    setDraft,
    editing,
    result,
    saved,
    showInput,
    onSave,
    onTest,
    onRemove,
    replace,
  };
}
