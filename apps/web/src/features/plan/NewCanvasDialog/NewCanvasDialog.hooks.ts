import { useEffect, useId, useState } from 'react';
import type { PlanPreset } from '@/ipc/client';
import { trimText } from '@/state/plan-text';
import { usePlansStore } from '@/state/use-plans-store';

/**
 * The new canvas's name, focused on open, and the preset it starts from;
 * Create makes the canvas with the preset's starter notes and opens it.
 */
export function useNewCanvasDialog(onClose: () => void, start: PlanPreset) {
  const create = usePlansStore(s => s.create);
  const [name, setName] = useState('');
  const [preset, setPreset] = useState<PlanPreset>(start);
  const [busy, setBusy] = useState(false);
  const field = useId();
  useEffect(() => {
    document.getElementById(field)?.focus();
  }, [field]);
  const trimmed = trimText(name);
  const submit = async () => {
    if (!trimmed || busy) return;
    setBusy(true);
    const id = await create(trimmed, preset);
    setBusy(false);
    if (id) onClose();
  };
  return {
    name,
    setName,
    preset,
    setPreset,
    field,
    busy,
    canCreate: trimmed.length > 0,
    submit,
  };
}
