/**
 * Real size and fix-ups as the UI shows and edits them: the labels, the
 * one line descriptions the core writes, adding and removing a fix-up in
 * the order they apply, and the before and after rows of a preview.
 */
import type {
  Fixup,
  FixupDefaults,
  FixupKind,
  FixupReport,
  ModelNumbers,
  ModelPrep,
  RealSize,
  SizeAxis,
  SizeUnit,
  TextureSize,
  WorkspaceConfig,
} from '@/ipc/client';
import { formatBytes, formatCount } from '@/lib/format';
import { extentOf, formatDims, type Extent } from '@/lib/real-size';

export const EMPTY_PREP: ModelPrep = { size: null, fixups: [] };

/** The kinds in the order they apply. */
export const FIXUP_ORDER: readonly FixupKind[] = [
  'apply-transforms',
  'remove-loose',
  'merge',
  'decimate',
  'shrink-textures',
  'origin-bottom',
];

export const FIXUP_LABEL: Record<FixupKind, string> = {
  'apply-transforms': 'Apply transforms',
  'remove-loose': 'Remove loose parts',
  merge: 'Merge objects',
  decimate: 'Decimate',
  'shrink-textures': 'Shrink textures',
  'origin-bottom': 'Origin to bottom centre',
};

/** What each kind does, for its tooltip. */
export const FIXUP_HINT: Record<FixupKind, string> = {
  'apply-transforms':
    'Bakes each object’s move, turn and scale into its mesh. Rigged meshes keep theirs.',
  'remove-loose': 'Drops stray specks: loose parts this share of the model’s size or smaller.',
  merge: 'Joins the meshes into one, or one per material, for fewer draw calls.',
  decimate: 'Collapses the mesh down to about this many triangles.',
  'shrink-textures': 'Scales every image down to this many pixels on its longest edge.',
  'origin-bottom': 'Moves the model so its bottom centre is the origin, standing on the floor.',
};

export const AXIS_NAME: Record<SizeAxis, string> = {
  height: 'Height',
  width: 'Width',
  depth: 'Depth',
  longest: 'Longest side',
};

const AXIS_WORD: Record<SizeAxis, string> = {
  height: 'high',
  width: 'wide',
  depth: 'deep',
  longest: 'on its longest side',
};

const trim = (v: number) => String(Number(v.toFixed(3)));

/** "45 cm high", "1.2 m wide", as the core says it. */
export function describeSize(size: RealSize): string {
  return `${trim(size.value ?? 0)} ${size.unit} ${AXIS_WORD[size.axis]}`;
}

/** "60k", "4.9k", "1.2M", as the inbox checks count. */
export function shortCount(n: number): string {
  const text =
    n >= 1e6
      ? `${(n / 1e6).toFixed(1)}M`
      : n >= 1e5
        ? `${(n / 1e3).toFixed(0)}k`
        : n >= 1e3
          ? `${(n / 1e3).toFixed(1)}k`
          : n.toFixed(0);
  return text.replace('.0k', 'k').replace('.0M', 'M');
}

/** "Decimate to 60k", "Shrink textures to 2048 px", as the core says it. */
export function describeFixup(f: Fixup): string {
  switch (f.op) {
    case 'decimate':
      return `Decimate to ${shortCount(f.triangles)}`;
    case 'shrink-textures':
      return `Shrink textures to ${f.maxSize} px`;
    case 'remove-loose':
      return `Remove loose parts under ${trim((f.minShare ?? 0) * 100)}%`;
    case 'merge':
      return f.byMaterial ? 'Merge objects by material' : 'Merge objects';
    default:
      return FIXUP_LABEL[f.op];
  }
}

export const isEmptyPrep = (p: ModelPrep | null | undefined): boolean =>
  !p || (p.size === null && p.fixups.length === 0);

const rank = (f: Fixup) => FIXUP_ORDER.indexOf(f.op);

/** `prep` with `fixup` in, replacing one of its kind, kept in the order they apply. */
export function withFixup(prep: ModelPrep, fixup: Fixup): ModelPrep {
  const rest = prep.fixups.filter(f => f.op !== fixup.op);
  return { ...prep, fixups: [...rest, fixup].sort((a, b) => rank(a) - rank(b)) };
}

export function withoutFixup(prep: ModelPrep, kind: FixupKind): ModelPrep {
  return { ...prep, fixups: prep.fixups.filter(f => f.op !== kind) };
}

export const withSize = (prep: ModelPrep, size: RealSize | null): ModelPrep => ({ ...prep, size });

/** True when `prep` already has exactly this fix-up. */
export function hasFixup(prep: ModelPrep, fixup: Fixup): boolean {
  return prep.fixups.some(f => JSON.stringify(f) === JSON.stringify(fixup));
}

const TEXTURE_PX: Record<TextureSize, number> = { '1K': 1024, '2K': 2048, '4K': 4096 };

/** What new fix-ups start from, the way the core works it out of the workspace settings. */
export function workspaceDefaults(config: WorkspaceConfig | null | undefined): FixupDefaults {
  return {
    triangles: config?.budgets.trisPerModel ?? 60_000,
    maxTexture: TEXTURE_PX[config?.compression.textureSize ?? '2K'],
    looseShare: 0.02,
  };
}

/** A new fix-up of `kind`, starting from the workspace budgets. */
export function defaultFixup(kind: FixupKind, d: FixupDefaults): Fixup {
  switch (kind) {
    case 'decimate':
      return { op: kind, triangles: d.triangles };
    case 'shrink-textures':
      return { op: kind, maxSize: d.maxTexture };
    case 'remove-loose':
      return { op: kind, minShare: d.looseShare ?? 0.02 };
    case 'merge':
      return { op: kind, byMaterial: false };
    default:
      return { op: kind };
  }
}

/** The size a new real size starts from: the measured height in centimetres or metres. */
export function startSize(measured: Extent | null, unit?: SizeUnit): RealSize {
  const h = measured ? measured[1] : 1;
  const use = unit ?? (h >= 1 ? 'm' : 'cm');
  const value = use === 'm' ? Number(h.toFixed(2)) : Number((h * 100).toFixed(1));
  return { axis: 'height', value: value > 0 ? value : 1, unit: use };
}

export const samePrep = (a: ModelPrep, b: ModelPrep): boolean =>
  JSON.stringify(a) === JSON.stringify(b);

export interface NumberRow {
  label: string;
  before: string;
  after: string;
  changed: boolean;
}

/** "0.62×1.58×0.31 m": tight, for a table cell. */
function dims(n: ModelNumbers): string {
  return n.min && n.max ? formatDims(extentOf(n.min, n.max)).replace(/ × /g, '×') : 'Unknown';
}

/** A preview's numbers before and after, row by row. */
export function reportRows(r: FixupReport): NumberRow[] {
  const rows: [string, string, string][] = [
    ['Triangles', formatCount(r.before.triangles), formatCount(r.after.triangles)],
    ['Objects', String(r.before.objects), String(r.after.objects)],
    [
      'Texture',
      r.before.maxTexture ? `${r.before.maxTexture} px` : 'None',
      r.after.maxTexture ? `${r.after.maxTexture} px` : 'None',
    ],
    ['Size', dims(r.before), dims(r.after)],
    ['File', formatBytes(r.bytesBefore ?? 0), formatBytes(r.bytesAfter ?? 0)],
  ];
  return rows.map(([label, before, after]) => ({
    label,
    before,
    after,
    changed: before !== after,
  }));
}

/** One step of a preview: "Decimate: 19,212 to 4,999 triangles, ratio 0.260". */
export function stepLine(step: FixupReport['steps'][number]): string {
  const tris =
    step.before.triangles !== step.after.triangles
      ? `${formatCount(step.before.triangles)} to ${formatCount(step.after.triangles)} triangles`
      : null;
  return [tris, step.note].filter(Boolean).join(', ') || 'Nothing to change';
}

/** The key of one preview: what it runs on and the plan. */
export function previewKey(target: string, plan: ModelPrep): string {
  return `${target}\n${JSON.stringify(plan)}`;
}
