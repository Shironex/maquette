/**
 * The rules of an export pack as the Library shows it: the default name,
 * the check's filters and counts, what waits for a person, the README the
 * pack will carry and the tree of what is inside. The core writes and
 * checks the files; this only reads its records.
 */
import type {
  AssetKind,
  PackFormat,
  PackItem,
  PackItemState,
  PackRecord,
  PackSettings,
  PackSource,
} from '@/ipc/client';
import { formatCount } from '@/lib/format';

export const KINDS: readonly AssetKind[] = ['prop', 'character', 'scene'];

export const KIND_LABEL: Record<AssetKind, string> = {
  prop: 'Props',
  character: 'Characters',
  scene: 'Scenes',
};

export const FORMAT_LABEL: Record<PackFormat, string> = {
  obj: 'OBJ, MTL, PNG',
  glb: 'GLB',
  both: 'Both',
};

export const TEXTURE_SIZES = [512, 1024, 2048, 4096] as const;

/** The smallest budget the core takes, as prepare's decimate does. */
export const MIN_BUDGET = 100;

/** The note the README and every Mixamo card carry. */
export const MIXAMO_NOTE =
  'Mixamo clips left out: Mixamo allows using them in your own work, not sharing them as files. The bones are kept, so the other side can add its own clips.';

/** A pack name as the core keeps it: lowercase words joined by dashes. */
export function packName(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 60);
}

/** What a pack of `source` is called before anyone names it. */
export function defaultName(source: PackSource, names: { collection?: string; project?: string }) {
  if (source.from === 'collection') return packName(names.collection ?? 'collection') || 'pack';
  if (source.from === 'project') return packName(names.project ?? source.project) || 'pack';
  return 'library-pack';
}

export type CheckFilter = 'all' | 'pass' | 'look' | 'failed' | 'running';

export const FILTER_LABEL: Record<CheckFilter, string> = {
  all: 'All',
  pass: 'Pass',
  look: 'To look at',
  failed: 'Failed',
  running: 'Running',
};

const IN_FILTER: Record<CheckFilter, readonly PackItemState[]> = {
  all: ['queued', 'running', 'pass', 'warn', 'fail', 'kept'],
  pass: ['pass', 'kept'],
  look: ['warn'],
  failed: ['fail'],
  running: ['running', 'queued'],
};

export function inFilter(item: PackItem, filter: CheckFilter): boolean {
  return IN_FILTER[filter].includes(item.state);
}

export function filterCounts(items: readonly PackItem[]): Record<CheckFilter, number> {
  const count = (f: CheckFilter) => items.filter(i => inFilter(i, f)).length;
  return {
    all: items.length,
    pass: count('pass'),
    look: count('look'),
    failed: count('failed'),
    running: count('running'),
  };
}

export const settled = (i: PackItem) => i.state !== 'queued' && i.state !== 'running';
export const passes = (i: PackItem) => i.state === 'pass' || i.state === 'kept';
/** The models a person has to answer before the zip. */
export const needsYou = (items: readonly PackItem[]) =>
  items.filter(i => i.state === 'warn' || i.state === 'fail');

/** "about 40 s left" from the models done so far, or null before one is. */
export function timeLeft(items: readonly PackItem[]): string | null {
  const took = items.map(i => i.tookMs).filter((ms): ms is number => ms !== null);
  const left = items.filter(i => !settled(i)).length;
  if (!took.length || !left) return null;
  const s = Math.round(((took.reduce((a, b) => a + b, 0) / took.length) * left) / 1000);
  return s < 60 ? `about ${Math.max(1, s)} s left` : `about ${Math.round(s / 60)} min left`;
}

/** "3:12", a run's length. */
export function formatTook(ms: number): string {
  const s = Math.round(ms / 1000);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

/** "29.9k" or "820". */
export function shortTris(n: number): string {
  return n >= 1000 ? `${(n / 1000).toFixed(1)}k` : String(n);
}

/** "3,000 tris · 0.92 m", the numbers under a check card. */
export function itemNumbers(item: PackItem): string {
  const tris = item.triangles ?? item.model.stats?.triangles ?? null;
  const parts = [tris === null ? null : `${formatCount(tris)} tris`];
  const height = item.heightM ?? item.model.heightM;
  if (height !== null) parts.push(`${height.toFixed(2)} m`);
  return parts.filter(Boolean).join(' · ');
}

/** What a waiting card says while it waits. */
export function itemNote(item: PackItem): string {
  if (item.note) return item.note;
  if (item.state === 'running') {
    const before = item.model.stats?.triangles ?? 0;
    return item.budget !== null && before > item.budget && !item.model.rigged
      ? `Decimating to ${formatCount(item.budget)}`
      : 'Writing and opening it again';
  }
  return 'Waiting for Blender';
}

/** The budget per kind, "props 3,000, characters as they are". */
export function budgetsLine(s: PackSettings): string {
  return KINDS.map(k => {
    const b = s.budgets.find(x => x.kind === k)?.triangles ?? null;
    return `${KIND_LABEL[k].toLowerCase()} ${b === null ? 'as they are' : formatCount(b)}`;
  }).join(', ');
}

/** The README the pack will carry, as the core writes it. */
export function readme(record: Pick<PackRecord, 'name' | 'settings' | 'items'>): string {
  const s = record.settings;
  const lines = [
    `${record.name}, ${record.items.length} models`,
    '',
    `Format    ${FORMAT_LABEL[s.format]}, Y up, ${s.realSize ? 'metres, real size' : 'metres'}`,
    `Pivot     ${s.pivotBottom ? 'bottom centre, on the floor' : 'as each model was made'}`,
    `Budget    ${budgetsLine(s)}`,
    `Material  ${s.oneMaterial ? 'named body (body_2 and on for more)' : 'as each model was made'}, ${
      s.texture ? `textures up to ${s.texture} px` : 'textures as they were'
    }`,
    '',
    "Import: drop the folder in. Keep each model's files together in its folder.",
  ];
  if (record.items.some(i => i.clipsLeftOut > 0 || i.model.mixamo)) lines.push('', MIXAMO_NOTE);
  return lines.join('\n');
}

export interface TreeRow {
  path: string;
  /** What the row holds, in a few words. */
  note: string;
  depth: number;
}

/** What is inside a finished pack: the files at its root, then a folder per model. */
export function packTree(record: PackRecord): TreeRow[] {
  const s = record.settings;
  const rows: TreeRow[] = [];
  const kept = record.items.filter(i => passes(i) || i.state === 'warn');
  if (s.readme) rows.push({ path: 'README.txt', note: 'How to import', depth: 1 });
  if (s.manifest) rows.push({ path: 'manifest.json', note: 'Names, sources, licences', depth: 1 });
  if (s.contactSheet) rows.push({ path: 'contact-sheet.png', note: 'Every model', depth: 1 });
  for (const item of kept) {
    const files = item.files.map(f => f.split('/').pop() ?? f);
    rows.push({ path: `${item.model.key}/`, note: `${files.length} files`, depth: 1 });
    for (const f of files) rows.push({ path: f, note: '', depth: 2 });
  }
  return rows;
}

/** The first models of the manifest, as the Done view previews it. */
export function manifestSnippet(record: PackRecord, n = 2): string {
  const models = record.items.filter(passes).slice(0, n);
  const body = models.map(i =>
    JSON.stringify({
      name: i.model.name,
      tris: i.triangles,
      height_m: i.heightM === null ? null : Math.round(i.heightM * 100) / 100,
      source: i.model.source,
    })
  );
  return `{\n  "pack": "${record.name}",\n  "models": [\n    ${body.join(',\n    ')}${
    record.items.length > n ? ',\n    ...' : ''
  }\n  ]\n}`;
}

/** "Garden tools went to 3,000 and Flower basket was exported again." */
export function doneLine(record: PackRecord): string {
  const passed = record.items.filter(passes).length;
  const all = record.items.length;
  const head =
    passed === all ? `${all} models passed` : `${passed} of ${all} models are in the pack`;
  return record.message ? `${head}. ${record.message}` : `${head}.`;
}

/** The triangles all the pack's models had before, and have now. */
export function triangleTotals(record: PackRecord): { before: number; after: number } {
  let before = 0;
  let after = 0;
  for (const i of record.items.filter(passes)) {
    before += i.model.stats?.triangles ?? 0;
    after += i.triangles ?? 0;
  }
  return { before, after };
}
