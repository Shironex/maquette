import { useState } from 'react';
import type { ModelFile } from '@/ipc/client';
import { artifactRows } from './artifactRows';

export const SHOWN = 4;

/** The rows to show, folded to the first few until Show all. */
export function useArtifactsList(models: readonly ModelFile[], busy: boolean) {
  const [all, setAll] = useState(false);
  const rows = artifactRows(models, busy);
  return {
    all,
    toggle: () => setAll(now => !now),
    total: rows.length,
    rows: all ? rows : rows.slice(0, SHOWN),
  };
}
