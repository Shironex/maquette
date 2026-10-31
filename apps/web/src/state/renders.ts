/**
 * Render styles: what each style is called, the settings the core starts
 * from, the file a style writes and the sums the views show. Pure; the
 * core plans and writes every file.
 */
import { formatBytes } from '@/lib/format';
import type {
  LoopFormat,
  RenderFile,
  RenderItem,
  RenderLook,
  RenderProblem,
  RenderRecord,
  RenderSettings,
  RenderStyle,
} from '@/ipc/client';

export const STYLES: readonly { id: RenderStyle; label: string; line: string }[] = [
  { id: 'blueprint', label: 'Blueprint', line: 'lines on blue · PNG' },
  { id: 'toon', label: 'Toon', line: 'flat colour, ink lines' },
  { id: 'clay', label: 'Clay', line: 'one colour, cavity' },
  { id: 'shaded', label: 'Shaded', line: 'as built, studio light' },
  { id: 'turntable', label: 'Turntable', line: 'WebP loop · GIF · MP4' },
  { id: 'contact-sheet', label: 'Contact sheet', line: '8 angles on one PNG' },
];

export const STYLE_LABEL = Object.fromEntries(STYLES.map(s => [s.id, s.label])) as Record<
  RenderStyle,
  string
>;

export const LOOK_OPTIONS = (['clay', 'toon', 'blueprint', 'shaded'] as const).map(value => ({
  value,
  label: STYLE_LABEL[value],
}));

/** The core's defaults, spelled out so a form always has a value to show. */
export const RENDER_DEFAULTS: Required<RenderSettings> = {
  camera: 'three-quarter',
  size: 2048,
  transparent: false,
  lineColor: '#b3d9ff',
  paperColor: null,
  lineWeight: 1.6,
  crease: 30,
  hiddenDashed: true,
  marks: true,
  titleBlock: true,
  clayColor: '#a89e92',
  format: 'webp',
  length: 3,
  width: 480,
  look: 'clay',
  tilt: 12,
  stillBackground: true,
  tightCrop: false,
  columns: 3,
  names: true,
};

export const FPS = 20;
export const TARGET_BYTES = 2_000_000;

/** The paper each look starts on, for the swatch. */
export const LOOK_PAPER: Record<RenderLook, string> = {
  blueprint: '#0f2a55',
  toon: '#f5f1ea',
  clay: '#e7e5df',
  shaded: '#5e5b56',
};

const LOOP_EXT: Record<LoopFormat, string> = { webp: 'webp', gif: 'gif', mp4: 'mp4' };
export const LOOP_LABEL: Record<LoopFormat, string> = {
  webp: 'WebP loop',
  gif: 'Animated GIF',
  mp4: 'MP4',
};

/** The file a style writes in an asset's folder. */
export function fileOf(style: RenderStyle, settings: RenderSettings): string {
  if (style === 'turntable') {
    return `turntable.${LOOP_EXT[settings.format ?? 'webp']}`;
  }
  return style === 'contact-sheet' ? 'angles.png' : `${style}.png`;
}

export function isLook(style: RenderStyle): style is RenderLook {
  return style !== 'turntable' && style !== 'contact-sheet';
}

/** How long Blender takes, roughly, for the note beside the Render button. */
export function secondsAbout(style: RenderStyle, settings: RenderSettings): number {
  const scale = Math.max(1, (settings.size ?? 2048) / 2048);
  if (style === 'turntable') {
    const frames = (settings.length ?? 3) * FPS;
    return Math.max(5, Math.round((frames * (settings.look === 'shaded' ? 2 : 0.4)) / 5) * 5);
  }
  if (style === 'shaded') return Math.round(20 * scale * scale);
  if (style === 'contact-sheet') return 15;
  return Math.round(6 * scale * scale);
}

/** Every file of a render, in the order they run. */
export function filesOf(record: RenderRecord): RenderFile[] {
  const files = record.items.flatMap(i => i.files);
  return record.sheet ? [...files, record.sheet] : files;
}

const COUNTED: ReadonlySet<RenderFile['state']> = new Set(['done', 'warn', 'failed']);

/** Files finished and files in all; what is left out counts as neither. */
export function progressOf(record: RenderRecord): { done: number; total: number } {
  const files = filesOf(record).filter(f => f.state !== 'skipped');
  return { done: files.filter(f => COUNTED.has(f.state)).length, total: files.length };
}

/** What Blender is on, or null when it is not on anything. */
export function runningOf(record: RenderRecord): { name: string; file: RenderFile } | null {
  for (const item of record.items) {
    const file = item.files.find(f => f.state === 'running');
    if (file) return { name: item.target.name, file };
  }
  return record.sheet?.state === 'running' ? { name: 'all of them', file: record.sheet } : null;
}

export type RowTone = 'done' | 'running' | 'queued' | 'warn' | 'failed';

/** The state chip of one asset's row. */
export function rowState(item: RenderItem): { label: string; tone: RowTone } {
  const running = item.files.find(f => f.state === 'running');
  const made = item.files.filter(f => COUNTED.has(f.state) && f.state !== 'failed').length;
  if (running) {
    const of = item.files.filter(f => f.state !== 'skipped').length;
    return {
      label: `${STYLE_LABEL[running.style].toLowerCase()} ${made + 1} of ${of}`,
      tone: 'running',
    };
  }
  if (item.files.some(f => f.state === 'failed')) return { label: 'failed', tone: 'failed' };
  const problem = item.files.find(f => f.problem);
  if (problem?.problem?.kind === 'no-texture') return { label: 'no texture', tone: 'warn' };
  if (problem) return { label: `${made} ${made === 1 ? 'file' : 'files'}, look`, tone: 'warn' };
  if (made > 0 && made === item.files.length) {
    return { label: `${made} ${made === 1 ? 'file' : 'files'}`, tone: 'done' };
  }
  return { label: 'queued', tone: 'queued' };
}

/** The status bar's line for a render still going. */
export function renderStatusLine(record: RenderRecord | null): string | null {
  if (record?.state !== 'running') return null;
  const { done, total } = progressOf(record);
  return `Rendering ${Math.min(done + 1, total)} of ${total} files`;
}

/** Turns "/a/b/exports/renders" and a folder into the path the panel shows. */
export function shortPath(dir: string, ...rest: string[]): string {
  const from = dir.lastIndexOf('/exports/');
  const start = from >= 0 ? dir.slice(from + 1) : dir;
  return [start, ...rest].join('/');
}

export interface ProblemCard {
  title: string;
  tone: 'danger' | 'warning';
}

/** The heading and colour of a problem, by what it asks of the person. */
export function problemCard(
  problem: RenderProblem,
  name: string,
  style: RenderStyle,
  bytes?: number | null
): ProblemCard {
  const label = STYLE_LABEL[style];
  switch (problem.kind) {
    case 'missing-tool':
      return {
        title: `${/ffmpeg/i.test(problem.message) ? 'ffmpeg' : 'Blender'} was not found`,
        tone: 'danger',
      };
    case 'over-target':
      return {
        title: `Turntable GIF came out at ${bytes ? formatBytes(bytes) : 'over 2 MB'}`,
        tone: 'warning',
      };
    case 'no-texture':
      return { title: 'Toon on a model with no texture', tone: 'warning' };
    case 'unreadable':
      return { title: `Blender cannot read ${name}`, tone: 'danger' };
    case 'out-of-memory':
    case 'failed':
      return { title: `${label} stopped on ${name}`, tone: 'danger' };
  }
}
