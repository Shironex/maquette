import { useMemo, useState } from 'react';
import type { CompressPreset, PipelineView } from '@/ipc/client';
import {
  editRow,
  estimateTransfer,
  hasEdits,
  presetRows,
  presetsToSave,
  withEdits,
  type PresetRow,
} from '@/state/presets';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { modelResults, modelsLine, sortRows, type ModelSort } from './modelResults';

const NO_EDITS = {};

/** The preset rows with the edits not applied yet, the open splits, the sort and Apply. */
export function usePresetsPanel(
  view: PipelineView,
  onSave: (presets: CompressPreset[]) => Promise<boolean>
) {
  const initial = useMemo(() => presetRows(view), [view]);
  // Edits live in the store until applied: a run's events or a trip to
  // another tab must not put the saved values back.
  const edits = usePipelineStore(s => s.presetEdits[view.project]) ?? NO_EDITS;
  const setEdits = usePipelineStore(s => s.setPresetEdits);
  const rows = withEdits(initial, edits);
  const [saving, setSaving] = useState(false);
  // Rows with a split open their extra line; the others start folded.
  const [open, setOpen] = useState<readonly string[]>(() =>
    initial.filter(r => r.dataTextureSize !== null).map(r => r.model)
  );
  const toggle = (model: string) =>
    setOpen(now => (now.includes(model) ? now.filter(m => m !== model) : [...now, model]));
  const dirty = hasEdits(initial, edits);
  const locks = rows.filter(r => r.lock);
  const update = (model: string, patch: Partial<PresetRow>) => {
    const row = rows.find(r => r.model === model);
    if (row) setEdits(view.project, editRow(initial, edits, { ...row, ...patch, custom: true }));
  };
  const save = async () => {
    setSaving(true);
    if (await onSave(presetsToSave(rows))) setEdits(view.project, {});
    setSaving(false);
  };
  const transfer = estimateTransfer(view, rows);
  const [sort, setSort] = useState<ModelSort>('name');
  const results = useMemo(() => modelResults(view), [view]);
  const shown = sortRows(rows, results, sort);
  return {
    rows: shown,
    results,
    summary: modelsLine(view),
    sort,
    setSort,
    open,
    toggle,
    dirty,
    locks,
    update,
    save,
    saving,
    transfer,
  };
}
