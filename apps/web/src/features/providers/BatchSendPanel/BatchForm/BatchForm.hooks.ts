import { useState } from 'react';
import type { PresetVariant } from '@/ipc/client';
import {
  batchBlocker,
  batchLines,
  batchProblem,
  variantsUsed,
  type BatchDraft,
} from '@/state/batch';
import { useAppStore } from '@/state/use-app-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useCatalog } from '../../PresetFields';

/** What the batch form does: edit the draft, send it, or leave for Settings. */
export function useBatchActions() {
  const setBatch = useProvidersStore(s => s.setBatch);
  const setItemVariant = useProvidersStore(s => s.setItemVariant);
  const setPreset = useProvidersStore(s => s.setBatchPreset);
  const setItemPreset = useProvidersStore(s => s.setItemPreset);
  const reset = useProvidersStore(s => s.resetBatchPresets);
  const send = useProvidersStore(s => s.sendBatch);
  const sending = useProvidersStore(s => s.sending);
  const confirming = useProvidersStore(s => s.batch?.confirming ?? false);
  const close = useProvidersStore(s => s.closeBatch);
  const setView = useAppStore(s => s.setView);
  const openSettings = () => {
    close();
    setView('settings');
  };
  return {
    setBatch,
    setItemVariant,
    setPreset,
    setItemPreset,
    reset,
    send,
    sending,
    confirming,
    close,
    openSettings,
  };
}

/**
 * What the form shows for a draft: the preset tab in view, the rows opened
 * for custom settings, the key, what blocks the send and the summary lines.
 */
export function useBatchView(draft: BatchDraft) {
  const keys = useProvidersStore(s => s.keys);
  const catalog = useCatalog();
  const used = variantsUsed(draft);
  const [tab, setTab] = useState<PresetVariant>(used[0] ?? 'prop');
  const shown: PresetVariant = used.includes(tab) ? tab : (used[0] ?? 'prop');
  const problem = batchProblem(draft, catalog);
  const blocker = batchBlocker(draft, keys, problem);
  const key = keys.find(k => k.service === draft.provider);
  const lines = batchLines(draft, catalog);
  const [open, setOpen] = useState<readonly string[]>([]);
  const toggle = (asset: string) =>
    setOpen(now => (now.includes(asset) ? now.filter(a => a !== asset) : [...now, asset]));
  return { catalog, used, shown, setTab, blocker, key, lines, open, toggle };
}
