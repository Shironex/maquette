/**
 * The Animations view's rules: filters, counts, card lines and the rest
 * pose drawing. Clips live once in the workspace library, skeleton only;
 * a model takes a clip by baking it, so nothing here touches a project.
 */
import type {
  AnimationClip,
  AnimationItem,
  ImportPlan,
  PlanClip,
  RestBone,
  SourceRig,
} from '@/ipc/client';
import { formatBytes } from '@/lib/format';

export const RIG_LABEL: Record<SourceRig, string> = {
  mixamo: 'Mixamo',
  tripo: 'Tripo',
  custom: 'Custom',
};

export const RIGS: readonly SourceRig[] = ['mixamo', 'tripo', 'custom'];

/** The pack single files go to. */
export const SINGLES = 'Singles';

export type AnimationLayout = 'grid' | 'list';

export interface AnimationFilters {
  query: string;
  pack: string | null;
  tag: string | null;
  rig: SourceRig | null;
  /** Only clips that travel. */
  rootMotion: boolean;
  /** Only clips that loop. */
  loops: boolean;
}

export const NO_ANIMATION_FILTERS: AnimationFilters = {
  query: '',
  pack: null,
  tag: null,
  rig: null,
  rootMotion: false,
  loops: false,
};

function matchesQuery(clip: AnimationClip, query: string): boolean {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  const text = [clip.name, clip.pack, ...clip.tags].join(' ').toLowerCase();
  return words.every(w => text.includes(w));
}

/** The clips the filters keep, newest first. */
export function filterClips(items: readonly AnimationItem[], f: AnimationFilters): AnimationItem[] {
  return items
    .filter(({ clip }) => {
      if (f.pack !== null && clip.pack !== f.pack) return false;
      if (f.tag !== null && !clip.tags.includes(f.tag)) return false;
      if (f.rig !== null && clip.rig.kind !== f.rig) return false;
      if (f.rootMotion && !clip.rootMotion) return false;
      if (f.loops && !clip.loops) return false;
      return matchesQuery(clip, f.query);
    })
    .sort((a, b) => b.clip.createdAt.localeCompare(a.clip.createdAt));
}

/** How many times each value turns up, most first, then by name. */
function countBy(values: readonly string[]): [string, number][] {
  const counts = new Map<string, number>();
  for (const v of values) counts.set(v, (counts.get(v) ?? 0) + 1);
  return [...counts].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
}

/** Packs by size, Singles after the real packs. */
export function packCounts(items: readonly AnimationItem[]): [string, number][] {
  const counts = countBy(items.map(i => i.clip.pack));
  return [...counts.filter(([p]) => p !== SINGLES), ...counts.filter(([p]) => p === SINGLES)];
}

export const tagCounts = (items: readonly AnimationItem[]) =>
  countBy(items.flatMap(i => i.clip.tags));

export function rigCounts(items: readonly AnimationItem[]): [SourceRig, number][] {
  return RIGS.map(r => [r, items.filter(i => i.clip.rig.kind === r).length]);
}

const seconds = (n: number | null) => `${(n ?? 0).toFixed(2)} s`;

/** "38 f · 1.27 s". */
export const lengthLine = (clip: Pick<AnimationClip, 'frames' | 'length'>) =>
  `${clip.frames} f · ${seconds(clip.length)}`;

/** "38 f · 1.27 s at 30 fps". */
export const lengthAtLine = (clip: AnimationClip) =>
  `${lengthLine(clip)} at ${Math.round(clip.fps ?? 0)} fps`;

/** "yes, 0.92 m to the left", or "no, in place". */
export function rootMotionLine(clip: Pick<AnimationClip, 'rootMotion'>): string {
  const root = clip.rootMotion;
  if (!root) return 'no, in place';
  const to = root.direction === 'forward' || root.direction === 'back' ? '' : 'to the ';
  return `yes, ${(root.metres ?? 0).toFixed(2)} m ${to}${root.direction}`;
}

/** "0.92 m left", for the stage's chip. */
export const travelLine = (clip: Pick<AnimationClip, 'rootMotion'>) =>
  clip.rootMotion
    ? `${(clip.rootMotion.metres ?? 0).toFixed(2)} m ${clip.rootMotion.direction}`
    : null;

/** "Singles · from Standing Dodge Left.fbx". */
export function originLine(clip: AnimationClip): string {
  const from = clip.origin.archive ?? clip.origin.file;
  return `${clip.pack} · from ${from}`;
}

/** "46 KB, no mesh". */
export const storedLine = (clip: AnimationClip) => `${formatBytes(clip.bytes ?? 0)}, no mesh`;

export interface AnimationStats {
  clips: number;
  packs: number;
  singles: number;
  disk: number;
}

export function animationStats(items: readonly AnimationItem[], disk: number): AnimationStats {
  const packs = new Set(items.map(i => i.clip.pack).filter(p => p !== SINGLES));
  return {
    clips: items.length,
    packs: packs.size,
    singles: items.filter(i => i.clip.pack === SINGLES).length,
    disk,
  };
}

export interface BoneLine {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

/**
 * The rest pose seen from the front, as lines from each bone's head to its
 * tail, fitted to a `size` square with a margin. Fingers and twist bones
 * draw too; they are short.
 */
export function restPoseLines(bones: readonly RestBone[], size: number): BoneLine[] {
  const points = bones.flatMap(b => [b.head, b.tail]).map(([x, y]) => [x ?? 0, y ?? 0] as const);
  if (points.length === 0) return [];
  const xs = points.map(p => p[0]);
  const ys = points.map(p => p[1]);
  const [minX, maxX, minY, maxY] = [
    Math.min(...xs),
    Math.max(...xs),
    Math.min(...ys),
    Math.max(...ys),
  ];
  const span = Math.max(maxX - minX, maxY - minY) || 1;
  const margin = size * 0.12;
  const scale = (size - margin * 2) / span;
  const offX = (size - (maxX - minX) * scale) / 2;
  const offY = (size - (maxY - minY) * scale) / 2;
  const at = (x: number | null, y: number | null): [number, number] => [
    offX + ((x ?? 0) - minX) * scale,
    size - (offY + ((y ?? 0) - minY) * scale),
  ];
  return bones.map(b => {
    const [x1, y1] = at(b.head[0], b.head[1]);
    const [x2, y2] = at(b.tail[0], b.tail[1]);
    return { x1, y1, x2, y2 };
  });
}

/** What the person chose for each clip in an import. */
export interface ImportChoices {
  /** Duplicates and revisions to add as clips of their own. */
  keepBoth: readonly string[];
  /** Clips to leave out. */
  skip: readonly string[];
}

/** Whether a clip goes in with these choices. */
export function goesIn(clip: PlanClip, choices: ImportChoices): boolean {
  if (choices.skip.includes(clip.key)) return false;
  return clip.status !== 'duplicate' || choices.keepBoth.includes(clip.key);
}

/** How many clips an import adds, and what they weigh, with these choices. */
export function importTotals(plan: ImportPlan, choices: ImportChoices) {
  const clips = plan.files.flatMap(f => f.clips).filter(c => goesIn(c, choices));
  return {
    add: clips.length,
    bytes: clips.reduce((sum, c) => sum + (c.bytes ?? 0), 0),
    leftOut: plan.leftOut,
  };
}

/** "53 FBX · 118 MB", or "1 clip · 1.2 MB" for a single file. */
export function fileLine(file: ImportPlan['files'][number]): string {
  const size = formatBytes(file.bytes ?? 0);
  if (file.archive) return `${file.fbx} FBX · ${size}`;
  return `${file.clips.length} ${file.clips.length === 1 ? 'clip' : 'clips'} · ${size}`;
}
