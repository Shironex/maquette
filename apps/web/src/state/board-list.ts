/**
 * The Board's list layout: the remembered choices per project, and the rows
 * it shows, filtered and sorted.
 */
import type { Asset, AssetKind, ModelFile, Stage } from '@/ipc/client';
import { formatAgo, formatBytes } from '@/lib/format';
import { cardNumbers, type CardNumbers } from './board';
import { STAGES } from './pipeline';

export type BoardLayout = 'board' | 'list';

export type SortKey =
  'name' | 'kind' | 'stage' | 'version' | 'tris' | 'size' | 'status' | 'changed';

export interface ListSort {
  key: SortKey;
  /** True for A to Z, small to large, oldest first. */
  ascending: boolean;
}

/** What the Board remembers per project on this machine. */
export interface BoardPrefs {
  layout: BoardLayout;
  sort: ListSort;
  /** One stage, or every stage. */
  stage: Stage | 'all';
  query: string;
}

export const DEFAULT_PREFS: BoardPrefs = {
  layout: 'board',
  sort: { key: 'changed', ascending: false },
  stage: 'all',
  query: '',
};

const SORT_KEYS: readonly SortKey[] = [
  'name',
  'kind',
  'stage',
  'version',
  'tris',
  'size',
  'status',
  'changed',
];

/** Remembered prefs as read back from storage: anything unknown falls back to the default. */
export function parsePrefs(value: unknown): BoardPrefs {
  if (typeof value !== 'object' || value === null) return DEFAULT_PREFS;
  const v = value as Record<string, unknown>;
  const sort = (typeof v.sort === 'object' && v.sort !== null ? v.sort : {}) as Record<
    string,
    unknown
  >;
  const key = SORT_KEYS.find(k => k === sort.key);
  return {
    layout: v.layout === 'list' ? 'list' : 'board',
    sort: key ? { key, ascending: sort.ascending === true } : DEFAULT_PREFS.sort,
    stage: STAGES.find(s => s === v.stage) ?? 'all',
    query: typeof v.query === 'string' ? v.query.slice(0, 120) : '',
  };
}

/** The storage key of a project's prefs. */
export function prefsKey(project: string): string {
  return `maquette.board.${project}`;
}

/** Where a size comes from: the shipped build, or the model file before the first Compress. */
export type SizeSource = 'shipped' | 'source';

/** One row of the list: the card's numbers plus what only the list shows. */
export interface ListRow {
  asset: Asset;
  n: CardNumbers;
  triangles: number | null;
  bytes: number | null;
  sizeSource: SizeSource | null;
  size: string | undefined;
  changedMs: number;
}

const stem = (file: string) =>
  (file.split('/').pop() ?? file).replace(/\.[^.]+$/, '').toLowerCase();

/**
 * The shipped bytes of an asset's newest model: the export named after its
 * file, else the one the pipeline labels with the asset and that version.
 */
export function shippedBytes(asset: Asset, models: readonly ModelFile[]): number | null {
  const latest = asset.versions[asset.versions.length - 1];
  if (!latest) return null;
  const own = stem(latest.file);
  const found =
    models.find(m => m.model.toLowerCase() === own) ??
    models.find(
      m => m.assetName === asset.name && (m.version === null || m.version === latest.version)
    );
  return found?.shippedBytes ?? null;
}

export function listRow(asset: Asset, models: readonly ModelFile[]): ListRow {
  const n = cardNumbers(asset);
  const stats = asset.versions[asset.versions.length - 1]?.stats ?? null;
  const shipped = shippedBytes(asset, models);
  const bytes = shipped ?? stats?.bytes ?? null;
  const sizeSource: SizeSource | null =
    shipped !== null ? 'shipped' : bytes !== null ? 'source' : null;
  return {
    asset,
    n,
    triangles: stats?.triangles ?? null,
    bytes,
    sizeSource,
    size: bytes !== null ? formatBytes(bytes) : undefined,
    changedMs: Date.parse(asset.updatedAt) || 0,
  };
}

/** Rows of one stage whose name, id or kind holds the query. */
export function filterRows(rows: readonly ListRow[], stage: BoardPrefs['stage'], query: string) {
  const words = query.trim().toLowerCase();
  return rows.filter(
    ({ asset }) =>
      (stage === 'all' || asset.stage === stage) &&
      (!words ||
        asset.name.toLowerCase().includes(words) ||
        asset.id.includes(words) ||
        asset.kind.includes(words))
  );
}

const STATUS_ORDER = ['failed', 'review', 'running', 'queued', 'cached', 'done'];
const KIND_ORDER: readonly AssetKind[] = ['scene', 'prop', 'character'];

function sortValue(row: ListRow, key: SortKey): string | number | null {
  switch (key) {
    case 'name':
      return row.asset.name.toLowerCase();
    case 'kind':
      return KIND_ORDER.indexOf(row.asset.kind);
    case 'stage':
      return STAGES.indexOf(row.asset.stage);
    case 'version':
      return row.n.version ?? null;
    case 'tris':
      return row.triangles;
    case 'size':
      return row.bytes;
    case 'status':
      return STATUS_ORDER.indexOf(row.asset.status);
    case 'changed':
      return row.changedMs;
  }
}

/** Sorted by the column; rows without a value go last either way, then by name. */
export function sortRows(rows: readonly ListRow[], sort: ListSort): ListRow[] {
  const dir = sort.ascending ? 1 : -1;
  return [...rows].sort((a, b) => {
    const x = sortValue(a, sort.key);
    const y = sortValue(b, sort.key);
    if (x === null || y === null) {
      if (x !== y) return x === null ? 1 : -1;
    } else if (x !== y) {
      return (x < y ? -1 : 1) * dir;
    }
    return a.asset.name.localeCompare(b.asset.name);
  });
}

/** Clicking a column: the same one flips, a new one starts in its natural direction. */
export function nextSort(sort: ListSort, key: SortKey): ListSort {
  if (sort.key === key) return { key, ascending: !sort.ascending };
  const textual = key === 'name' || key === 'kind' || key === 'stage' || key === 'status';
  return { key, ascending: textual };
}

/** "3 h ago", for the Last change column. */
export function changedLabel(row: ListRow, nowMs: number): string {
  return formatAgo(row.changedMs, nowMs);
}
