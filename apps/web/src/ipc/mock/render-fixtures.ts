/**
 * Render styles in mock mode: the pictures a preview answers with (real
 * headless Blender renders of the crystal lantern, bundled under
 * `assets/mock/renders`), the targets of a source and the renders
 * `?render=running` and `?render=failures` open with.
 */
import type {
  LibraryView,
  MovingBox,
  PreviewShot,
  ProjectManifest,
  RenderFile,
  RenderItem,
  RenderProblem,
  RenderRecord,
  RenderShot,
  RenderStyle,
  RenderTarget,
} from '../client';
import { modelsOfItems, modelsOfProject } from './pack-fixtures';

/** The folder the bundled pictures are served under. */
export const previewDir = (root: string) => `${root}/render-previews`;

const FRAME_NUMBERS = [1, 9, 17, 25, 33, 41, 49, 57];
const TURN_FRAME: Record<number, number> = { 90: 16, 180: 31, 270: 46 };
const MOVING: MovingBox = { x: 94, y: 58, w: 292, h: 396, canvas: 480 };

/** Every picture of the set, for `install.ts` to register. */
export const RENDER_FILES: readonly string[] = [
  'blueprint.webp',
  'toon.webp',
  'clay.webp',
  'shaded.webp',
  'contact-sheet.webp',
  ...[...FRAME_NUMBERS, 16, 31, 46].map(n => `turn-${n}.webp`),
];

/** Loop sizes the sample's numbers came from, in bytes. */
const LOOP_BYTES = { webp: 210_000, gif: 1_320_000, mp4: 310_000 } as const;
export const BIG_GIF = 6_800_000;

export function shotOf(
  root: string,
  shot: RenderShot,
  format: keyof typeof LOOP_BYTES,
  width: number,
  big: boolean,
  tight: boolean
): PreviewShot {
  const dir = previewDir(root);
  const loop = shot.style === 'turntable' || shot.style === 'contact-sheet';
  if (loop) {
    const scale = (width / 480) ** 2 * (tight ? 0.86 : 1);
    return {
      ...shot,
      file: `${dir}/turn-1.webp`,
      seconds: 3.5,
      frames: FRAME_NUMBERS.map(n => `${dir}/turn-${n}.webp`),
      moving: tight ? { ...MOVING, x: 102, y: 66, w: 276, h: 380 } : MOVING,
      estimateBytes:
        big && format === 'gif'
          ? BIG_GIF
          : shot.style === 'turntable'
            ? Math.round(LOOP_BYTES[format] * scale)
            : null,
    };
  }
  const turned = TURN_FRAME[shot.turn];
  return {
    ...shot,
    file: turned ? `${dir}/turn-${turned}.webp` : `${dir}/${shot.style}.webp`,
    seconds: shot.size > 200 ? 1.4 : 0.4,
    frames: [],
    moving: null,
    estimateBytes: null,
  };
}

/** The models that have nothing to colour them with, so toon is left out. */
const NO_COLOUR = new Set(['observatory-arch']);

function target(
  key: string,
  name: string,
  model: {
    version: number;
    file: string;
    thumbnail: string | null;
    heightM: number | null;
    stats: { triangles?: number } | null;
  }
): RenderTarget {
  return {
    key,
    name,
    folder: name.toLowerCase().replace(/[^a-z0-9]+/g, '-'),
    version: model.version,
    file: model.file,
    thumbnail: model.thumbnail,
    heightM: model.heightM,
    triangles: model.stats?.triangles ?? 0,
    hasColour: !NO_COLOUR.has(key),
    unreadable: null,
  };
}

export function targetsOfProject(project: ProjectManifest, dir: string): RenderTarget[] {
  return modelsOfProject(project, dir)
    .filter(m => m.kind !== 'scene')
    .map(m =>
      target(m.asset ?? m.key, m.name, m.asset === 'crystal-lantern' ? { ...m, heightM: 1.84 } : m)
    );
}

export function targetsOfLibrary(view: LibraryView, ids: readonly string[]): RenderTarget[] {
  const items = view.items.filter(i => ids.length === 0 || ids.includes(i.entry.id));
  return modelsOfItems(items).map(m => target(m.entry ?? m.key, m.name, m));
}

const EXT: Partial<Record<RenderStyle, string>> = { turntable: 'webp' };

export function fileName(style: RenderStyle, ext?: string): string {
  if (style === 'contact-sheet') return 'angles.png';
  if (style === 'turntable') return `turntable.${ext ?? EXT.turntable}`;
  return `${style}.png`;
}

export function fileFor(
  dir: string,
  folder: string,
  style: RenderStyle,
  ext?: string,
  state: RenderFile['state'] = 'queued',
  problem: RenderProblem | null = null
): RenderFile {
  const done = state === 'done' || state === 'warn';
  return {
    style,
    file: `${dir}/${folder}/${fileName(style, ext)}`,
    state,
    bytes: done ? (style === 'turntable' ? 1_320_000 : 840_000) : null,
    seconds: done ? 6.2 : null,
    problem,
    moving: style === 'turntable' && done ? MOVING : null,
  };
}

export const NO_TEXTURE = (name: string): RenderProblem => ({
  kind: 'no-texture',
  message: `${name} has no texture, so toon would come out flat grey. Clay and blueprint still work.`,
  webpBytes: null,
});

const OVER = (): RenderProblem => ({
  kind: 'over-target',
  message:
    'Over the 2 MB target. The moving area fills the frame. Crop to the model, drop to 360 px, or save a WebP loop instead (about 1.1 MB).',
  webpBytes: 1_100_000,
});

const NO_MEMORY = (): RenderProblem => ({
  kind: 'out-of-memory',
  message:
    'The line pass ran out of memory at 4096 px (380k edges). Try 2048 px or Edges from crease 45°.',
  webpBytes: null,
});

const MISSING_BLENDER = (): RenderProblem => ({
  kind: 'missing-tool',
  message:
    'Render styles use headless Blender. Set its path in Settings, Toolchain, then render again.',
  webpBytes: null,
});

function base(id: string, project: string, dir: string, items: RenderItem[]): RenderRecord {
  return {
    version: 1,
    id,
    source: { from: 'project', project },
    label: 'of Frostveil',
    dir,
    settings: {},
    state: 'done',
    items,
    sheet: null,
    createdAt: new Date(Date.parse('2026-10-30T12:00:00Z')).toISOString(),
    finishedAt: null,
    message: null,
  };
}

/** `?render=failures`: one render of the lantern that hit every problem. */
export function failedRecord(project: string, dir: string, targets: RenderTarget[]): RenderRecord {
  const first = targets.find(t => t.key === 'crystal-lantern') ?? targets[0]!;
  const f = (
    style: RenderStyle,
    ext: string | undefined,
    problem: RenderProblem,
    state: RenderFile['state']
  ) => fileFor(dir, first.folder, style, ext, state, problem);
  return base('render-failed', project, dir, [
    {
      target: first,
      files: [
        f('shaded', undefined, MISSING_BLENDER(), 'failed'),
        { ...f('turntable', 'gif', OVER(), 'warn'), bytes: BIG_GIF },
        f('toon', undefined, NO_TEXTURE(first.name), 'skipped'),
        f('blueprint', undefined, NO_MEMORY(), 'failed'),
      ],
    },
  ]);
}

const RUNNING_STYLES: readonly RenderStyle[][] = [
  ['blueprint', 'turntable'],
  ['clay', 'turntable'],
  ['clay', 'toon'],
  ['clay', 'toon'],
  ['clay', 'turntable'],
  ['clay', 'blueprint'],
  ['clay'],
];

/** `?render=running`: a render held part way, one file running. */
export function runningRecord(project: string, dir: string, targets: RenderTarget[]): RenderRecord {
  const picked = targets.slice(0, RUNNING_STYLES.length);
  let left = picked.length;
  let running = false;
  const items = picked.map((t, row): RenderItem => {
    const styles = RUNNING_STYLES[row] ?? ['clay'];
    return {
      target: t,
      files: styles.map(style => {
        if (left > 0) {
          left -= 1;
          return fileFor(dir, t.folder, style, 'gif', 'done');
        }
        if (!running) {
          running = true;
          return fileFor(dir, t.folder, style, 'gif', 'running');
        }
        return fileFor(dir, t.folder, style, 'gif');
      }),
    };
  });
  const record = base('render-running', project, dir, items);
  return {
    ...record,
    state: 'running',
    sheet: { ...fileFor(dir, '.', 'contact-sheet'), file: `${dir}/frostveil_contact.png` },
  };
}
