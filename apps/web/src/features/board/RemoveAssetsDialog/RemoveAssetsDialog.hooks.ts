import { useState } from 'react';
import { errorMessage } from '@/ipc/client';
import { MAX_REMOVE, removePlan } from '@/state/trash';
import { useTrashStore } from '@/state/use-trash-store';
import type { RemoveAssetsDialogProps } from './RemoveAssetsDialog.types';

/** What the remove stops and waits for, and the remove itself. */
export function useRemoveAssetsDialog({ project, assets, jobs, onDone }: RemoveAssetsDialogProps) {
  const remove = useTrashStore(s => s.remove);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const plan = removePlan(assets, jobs);
  const blocked = new Set(plan.running.map(j => j.asset));
  const removable = assets.filter(a => !blocked.has(a.id));
  const tooMany = assets.length > MAX_REMOVE;

  const confirm = async () => {
    if (busy || removable.length === 0 || tooMany) return;
    setBusy(true);
    setError(null);
    try {
      await remove(
        project,
        assets.map(a => a.id)
      );
      onDone();
    } catch (err) {
      setError(errorMessage(err));
      setBusy(false);
    }
  };

  return { plan, removable, tooMany, busy, error, confirm };
}
