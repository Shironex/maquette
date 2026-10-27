/**
 * The Inspector's view logic: which two sides the compare slider shows, the
 * lines the stats panel prints, the budget bars, the frame time graph and
 * the camera flight between stops. Plain functions, so the tests need no
 * renderer.
 */
import type {
  CameraStopDef,
  Comparison,
  CompareSide,
  InspectModel,
  InspectView,
  ModelPreset,
  TextureRow,
} from '@/ipc/client';
import { fileName } from './pipeline';
import { formatBytes, formatK } from '@/lib/format';

export type Vec3 = [number, number, number];
type Maybe = number | null;

/** Floats cross the IPC as `number | null` (a NaN would be null); this reads them as numbers. */
export const num = (v: Maybe | undefined): number => v ?? 0;
export const vec3 = (v: readonly [Maybe, Maybe, Maybe]): Vec3 => [num(v[0]), num(v[1]), num(v[2])];

/** A camera stop with plain numbers. */
export interface Stop {
  name: string;
  position: Vec3;
  target: Vec3;
  fov: number;
  poster: string | null;
}

export function stopsOf(view: InspectView): Stop[] {
  return view.stops.stops.map((s: CameraStopDef) => ({
    name: s.name,
    position: vec3(s.position),
    target: vec3(s.target),
    fov: s.fov ?? 35,
    poster: s.poster,
  }));
}

/** 60 fps. */
export const FRAME_TARGET_MS = 1000 / 60;
/** One graph sample per this many milliseconds. */
export const SAMPLE_EVERY_MS = 250;
/** 30 seconds of samples. */
export const MAX_SAMPLES = 120;
/** How long the camera takes to fly to a stop. */
export const FLIGHT_MS = 1100;

export interface SidePick {
  comparison: Comparison;
  left: CompareSide | null;
  right: CompareSide;
}

/** The comparison and the two sides to show: the newest two unless picked. */
export function pickSides(
  view: InspectView,
  comparisonId: string | null,
  leftId: string | null,
  rightId: string | null
): SidePick | null {
  const comparison =
    view.comparisons.find(c => c.id === comparisonId) ?? view.comparisons[0] ?? null;
  if (!comparison || comparison.sides.length === 0) return null;
  const sides = comparison.sides;
  const right = sides.find(s => s.id === rightId) ?? sides[sides.length - 1]!;
  const left =
    sides.find(s => s.id === leftId && s.id !== right.id) ??
    [...sides].reverse().find(s => s.id !== right.id) ??
    null;
  return { comparison, left, right };
}

/** The models of a side, in order, skipping ids the view does not know. */
export function sideModels(view: InspectView, side: CompareSide | null): InspectModel[] {
  if (!side) return [];
  return side.members
    .map(m => view.models.find(model => model.id === m.model))
    .filter((m): m is InspectModel => Boolean(m));
}

/** "v6 vs v7", or just "Shipped" when there is nothing to compare with. */
export function versusLabel(pick: SidePick): string {
  return pick.left ? `${pick.left.label} vs ${pick.right.label}` : pick.right.label;
}

/** "2048² · 1.2 MB", "2048 × 1024 · 640 KB", "1.2 MB" without dimensions. */
export function textureLine(t: TextureRow): string {
  const size = formatBytes(num(t.bytes));
  if (!t.width || !t.height) return size;
  const dims = t.width === t.height ? `${t.width}²` : `${t.width} × ${t.height}`;
  return `${dims} · ${size}`;
}

/**
 * "Base color", "Normal", "Metal and roughness", or the image name. When two
 * textures share a use, the image name tells them apart.
 */
export function textureLabel(t: TextureRow, all: readonly TextureRow[] = [t]): string {
  const use = t.uses[0];
  if (!use) return t.name;
  const label = use[0]!.toUpperCase() + use.slice(1);
  const shared = all.filter(o => o.uses[0] === use).length > 1;
  return shared ? `${label}, ${t.name}` : label;
}

/** A swatch tone per texture use, as the artboard shows them. */
export function textureTone(t: TextureRow): string {
  switch (t.uses[0]) {
    case 'base color':
      return 'var(--color-clay)';
    case 'normal':
      return '#8a86d6';
    case 'metal and roughness':
    case 'occlusion':
      return '#6f7a63';
    case 'emissive':
      return '#d9b45f';
    default:
      return 'var(--color-line-strong)';
  }
}

/** "character.glb · 2 clips" */
export function modelSubtitle(m: InspectModel): string {
  const file = fileName(m.path);
  const clips = m.animations.length;
  return clips > 0 ? `${file} · ${clips} ${clips === 1 ? 'clip' : 'clips'}` : file;
}

export interface StatCard {
  label: string;
  value: string;
  over?: boolean;
}

/** The four cards: triangles (against the per model budget), textures, GPU memory, transfer. */
export function statCards(m: InspectModel, trisPerModel: number): StatCard[] {
  return [
    { label: 'Triangles', value: formatK(m.triangles), over: m.triangles > trisPerModel },
    { label: 'Textures', value: String(m.textures.length) },
    { label: 'GPU memory', value: formatBytes(num(m.gpuBytes)) },
    { label: 'Transfer', value: formatBytes(num(m.bytes)) },
  ];
}

export function codecLabel(codec: ModelPreset['codec']): string {
  return codec === 'etc1s' ? 'KTX2 ETC1S' : 'UASTC';
}

/** What the lock note says under the compression controls. */
export function presetNote(m: InspectModel): string {
  const p = m.preset;
  if (p.lock) return `Locked: ${p.lock}`;
  if (!p.hasRow)
    return `No preset row yet, so the workspace defaults apply at ${p.textureSize} px.`;
  return `Textures compress to ${p.textureSize} px on the next run.`;
}

export interface BudgetRow {
  model: string;
  label: string;
  mb: number;
  targetMb: number;
  over: boolean;
  /**
   * Where the target comes from: the model's own row in budgets.json, an
   * even share of the scene target, or (the only model without a row) the
   * whole scene target, which the Scene total already shows.
   */
  target: 'model' | 'share' | 'scene';
}

export function budgetRows(view: InspectView): BudgetRow[] {
  const open = view.budgets.filter(b => !b.explicit).length;
  const named = view.budgets.length - open;
  return view.budgets.map(b => ({
    model: b.model,
    label: b.label,
    mb: num(b.bytes) / 1e6,
    targetMb: num(b.targetMb),
    over: num(b.bytes) / 1e6 > num(b.targetMb),
    target: b.explicit ? 'model' : open === 1 && named === 0 ? 'scene' : 'share',
  }));
}

/** The tooltip that says which target a budget row is measured against. */
export function budgetTargetNote(row: BudgetRow, sceneMb: number): string {
  if (row.target === 'model')
    return `Its own target in budgets.json: ${formatMb(row.targetMb)} MB.`;
  if (row.target === 'share')
    return `No target of its own: an even share of the ${formatMb(sceneMb)} MB scene target. Set one in budgets.json.`;
  return `No target of its own, so the ${formatMb(sceneMb)} MB scene target applies (see Scene total). Set one in budgets.json.`;
}

/** "1.5" for megabytes in the budget rows, two decimals below 0.1. */
export function formatMb(mb: number): string {
  if (mb > 0 && mb < 0.1) return mb.toFixed(2);
  return mb.toFixed(1);
}

/** The model a side's budget row stands for, to select it. */
export function modelForBudget(
  view: InspectView,
  side: CompareSide | null,
  stem: string
): InspectModel | null {
  return sideModels(view, side).find(m => m.model === stem) ?? null;
}

/** Rolling frame samples, newest last, capped to `MAX_SAMPLES`. */
export function pushSample(samples: readonly number[], ms: number): number[] {
  const next = [...samples, ms];
  return next.length > MAX_SAMPLES ? next.slice(next.length - MAX_SAMPLES) : next;
}

/** The graph's top: the target with headroom, or the slowest sample. */
export function graphCeiling(samples: readonly number[]): number {
  return Math.max(FRAME_TARGET_MS * 1.25, ...samples.map(s => s * 1.1));
}

/** SVG polyline points for `samples` in a `width` by `height` box, right aligned. */
export function graphPoints(samples: readonly number[], width: number, height: number): string {
  if (samples.length === 0) return '';
  const ceiling = graphCeiling(samples);
  const step = width / (MAX_SAMPLES - 1);
  const start = width - (samples.length - 1) * step;
  return samples
    .map((ms, i) => {
      const x = start + i * step;
      const y = height - (Math.min(ms, ceiling) / ceiling) * height;
      return `${round(x)},${round(y)}`;
    })
    .join(' ');
}

/** Where the 60 fps line sits in a graph `height` tall. */
export function targetY(samples: readonly number[], height: number): number {
  return round(height - (FRAME_TARGET_MS / graphCeiling(samples)) * height);
}

function round(n: number): number {
  return Math.round(n * 10) / 10;
}

export function average(samples: readonly number[]): number {
  return samples.length === 0 ? 0 : samples.reduce((a, b) => a + b, 0) / samples.length;
}

export function easeInOutCubic(t: number): number {
  const x = Math.min(1, Math.max(0, t));
  return x < 0.5 ? 4 * x * x * x : 1 - (-2 * x + 2) ** 3 / 2;
}

export interface CameraPose {
  position: [number, number, number];
  target: [number, number, number];
  fov: number;
}

const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
const lerp3 = (
  a: readonly [number, number, number],
  b: readonly [number, number, number],
  t: number
): [number, number, number] => [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)];

/** The camera `elapsed` ms into a flight from `from` to `stop`, eased. */
export function flight(from: CameraPose, stop: Stop, elapsed: number): CameraPose {
  const t = easeInOutCubic(elapsed / FLIGHT_MS);
  return {
    position: lerp3(from.position, stop.position, t),
    target: lerp3(from.target, stop.target, t),
    fov: lerp(from.fov, stop.fov, t),
  };
}

/** "From stops.json · click to fly" and friends. */
export function stopsHelp(view: InspectView): string {
  const s = view.stops;
  switch (s.source) {
    case 'project-file':
      return `From ${s.file ?? 'stops.json'} · click to fly`;
    case 'layout':
      return 'From the scene layout · click to fly';
    case 'posters':
      return 'From the poster cameras · click to fly';
    case 'model-cameras':
      return `From the cameras in ${s.file ?? 'the scene'} · click to fly`;
    default:
      return 'No stops yet';
  }
}
