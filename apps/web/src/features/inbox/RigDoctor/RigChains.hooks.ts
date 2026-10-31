import { useState } from 'react';
import type { RigChain, RigChainEdit, RigRecord } from '@/ipc/client';
import { useRigStore } from '@/state/use-rig-store';
import { chainChanged, chainSide } from '@/state/rig';

/** Editing a creature's chains: the rows in the works, and the person's word on them. */
export function useChainEditor(record: RigRecord) {
  const confirmChains = useRigStore(s => s.confirmChains);
  const busy = useRigStore(s => s.busy);
  const [draft, setDraft] = useState<Record<string, RigChainEdit> | null>(null);
  const edits = Object.values(draft ?? {});
  const rows = record.creature?.chains ?? [];
  const change = (row: RigChain, part: Partial<RigChainEdit>) =>
    setDraft(d => {
      const key = row.key ?? '';
      const was = d?.[key] ?? { key, readAs: null, side: null };
      return { ...d, [key]: { ...was, ...part } };
    });
  const save = async () => {
    await confirmChains(edits);
    setDraft(null);
  };
  return {
    editing: draft !== null,
    busy,
    changed: edits.some(e => chainChanged(e, rows)),
    start: () => setDraft({}),
    cancel: () => setDraft(null),
    readAsOf: (row: RigChain) =>
      draft?.[row.key ?? '']?.readAs ?? (row.readAs === 'junk' ? 'junk' : 'chain'),
    sideOf: (row: RigChain) => draft?.[row.key ?? '']?.side ?? chainSide(row),
    setRead: (row: RigChain, readAs: string) => change(row, { readAs }),
    setSide: (row: RigChain, side: string) => change(row, { side }),
    save,
  };
}
