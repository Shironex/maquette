import { useState } from 'react';
import type { PlanSummary } from '@/ipc/client';
import { usePlansStore } from '@/state/use-plans-store';

/** Remove moves the canvas file into the workspace trash, then closes. */
export function useRemoveCanvasDialog(plan: PlanSummary, onClose: () => void) {
  const remove = usePlansStore(s => s.remove);
  const [busy, setBusy] = useState(false);
  return {
    busy,
    confirm: async () => {
      setBusy(true);
      const done = await remove(plan.id);
      setBusy(false);
      if (done) onClose();
    },
  };
}
