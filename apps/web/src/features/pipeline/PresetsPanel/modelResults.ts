/**
 * What each model weighs before and after Compress, beside its preset row:
 * the totals line, and the orders the Models table sorts in.
 */
import type { PipelineView } from '@/ipc/client';
import { formatBytes } from '@/lib/format';
import type { PresetRow } from '@/state/presets';

export type ModelSort = 'name' | 'shipped' | 'raw';

export const SORTS: readonly { value: ModelSort; label: string }[] = [
  { value: 'name', label: 'By name' },
  { value: 'shipped', label: 'By shipped size' },
  { value: 'raw', label: 'By raw size' },
];

export interface ModelResult {
  raw: number | null;
  shipped: number | null;
}

/** The raw export and the compressed file of each model, by its GLB stem. */
export function modelResults(view: Pick<PipelineView, 'models'>): Map<string, ModelResult> {
  return new Map(view.models.map(m => [m.model, { raw: m.rawBytes, shipped: m.shippedBytes }]));
}

/** "24 models · 27.4 MB → 2.3 MB", or what is missing yet. */
export function modelsLine(view: Pick<PipelineView, 'models'>): string {
  const count = view.models.length;
  const noun = `${count} ${count === 1 ? 'model' : 'models'}`;
  if (count === 0) return 'No models yet';
  const raw = view.models.reduce((sum, m) => sum + (m.rawBytes ?? 0), 0);
  const shipped = view.models.reduce((sum, m) => sum + (m.shippedBytes ?? 0), 0);
  if (raw === 0) return `${noun}, not exported yet`;
  if (shipped === 0) return `${noun} · ${formatBytes(raw)} raw, not compressed yet`;
  return `${noun} · ${formatBytes(raw)} → ${formatBytes(shipped)}`;
}

/** One model's result as the table cell says it. */
export function resultText(result: ModelResult | undefined): string {
  if (!result?.raw) return 'not exported';
  if (!result.shipped) return `${formatBytes(result.raw)} raw`;
  return formatBytes(result.shipped);
}

/** The rows in the asked order; sizes go largest first, unknown sizes last, ties by name. */
export function sortRows(
  rows: readonly PresetRow[],
  results: ReadonlyMap<string, ModelResult>,
  sort: ModelSort
): PresetRow[] {
  const byName = (a: PresetRow, b: PresetRow) => a.label.localeCompare(b.label);
  if (sort === 'name') return [...rows].sort(byName);
  const size = (r: PresetRow) => results.get(r.model)?.[sort] ?? -1;
  return [...rows].sort((a, b) => size(b) - size(a) || byName(a, b));
}
