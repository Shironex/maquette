import type { ModelFile } from '@/ipc/client';
import { formatBytes } from '@/lib/format';

export interface ArtifactRow {
  key: string;
  /** The file, as the step wrote it. */
  file: string;
  /** "4.6 → 1.1 MB", "new · 0.4 MB", or what is still to come. */
  size: string;
  /** True when the file is not finished yet. */
  pending: boolean;
}

/** One file's sizes as a line: raw to shipped when both are known. */
export function sizeText(m: Pick<ModelFile, 'rawBytes' | 'shippedBytes'>, busy: boolean): string {
  const { rawBytes: raw, shippedBytes: shipped } = m;
  if (raw && shipped) return `${formatBytes(raw)} → ${formatBytes(shipped)}`;
  if (shipped) return `new · ${formatBytes(shipped)}`;
  if (raw) return busy ? `${formatBytes(raw)} → compressing...` : `${formatBytes(raw)} raw`;
  return busy ? 'exporting...' : 'not exported';
}

/** The generated model files of a project, raw export to compressed, in name order. */
export function artifactRows(models: readonly ModelFile[], busy: boolean): ArtifactRow[] {
  return [...models]
    .sort((a, b) => a.model.localeCompare(b.model))
    .map(m => ({
      key: m.model,
      file: `${m.model}.glb`,
      size: sizeText(m, busy),
      pending: !m.rawBytes || !m.shippedBytes,
    }));
}
