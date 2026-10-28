import { useState } from 'react';
import type { RefWarning } from '@/ipc/client';

/** Whether the row's ref warnings are spread out, and what its warning line says. */
export function useItemWarnings(warnings: readonly RefWarning[] | null) {
  const [showWarnings, setShowWarnings] = useState(false);
  const warned = warnings && warnings.length > 0 ? warnings : null;
  return {
    showWarnings,
    toggleWarnings: () => setShowWarnings(!showWarnings),
    warned,
    warnedTitle: warned?.map(w => w.title).join('\n') ?? '',
  };
}
