import { useEffect, useId, useState } from 'react';
import { trimText } from '@/state/plan-text';
import { usePlansStore } from '@/state/use-plans-store';

/** The new canvas's name, focused on open; Create makes the canvas and opens it. */
export function useNewCanvasDialog(onClose: () => void) {
  const create = usePlansStore(s => s.create);
  const [name, setName] = useState('');
  const [busy, setBusy] = useState(false);
  const field = useId();
  useEffect(() => {
    document.getElementById(field)?.focus();
  }, [field]);
  const trimmed = trimText(name);
  const submit = async () => {
    if (!trimmed || busy) return;
    setBusy(true);
    const id = await create(trimmed);
    setBusy(false);
    if (id) onClose();
  };
  return { name, setName, field, busy, canCreate: trimmed.length > 0, submit };
}
