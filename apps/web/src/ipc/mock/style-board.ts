/**
 * The project style board in mock mode, as `concepts::style` keeps it:
 * images copied into `style/`, a line, a palette, a revision that goes up
 * on each real change. `?style=board` seeds Frostveil with one and marks
 * which of its directions carried it.
 */
import type { Direction, StyleBoard, StyleBoardChange, StyleImage } from '../client';
import { clock } from '@/lib/clock';
import {
  EMPTY_STYLE,
  MAX_PALETTE,
  MAX_STYLE_IMAGES,
  MAX_STYLE_LINE,
  isStyleEmpty,
  normalizeColour,
} from '@/state/lab-style';

export interface StyleBoardOptions {
  projectDir: (project: string) => string;
  files?: { alias: (path: string, from: string) => void };
  fail: (code: string, message: string) => never;
  /** `board`: Frostveil has a style board. */
  state?: string | null;
}

/** The seeded board's images: the copy in `style/` and the concept or ref it came from. */
const SEEDED: readonly StyleImage[] = [
  {
    file: 'style/veil_gate_kit_01.png',
    source: 'concepts/veil-gate-kit/veil-gate-kit_01.png',
  },
  { file: 'style/frozen_lake_concept.png', source: 'refs/frozen-lake-concept.png' },
  {
    file: 'style/elsheria_sheets_02.png',
    source: 'concepts/elsheria-sheets/elsheria-sheets_02.png',
  },
];

export function frostveilStyleBoard(): StyleBoard {
  return {
    version: 1,
    revision: 3,
    line: 'frosted glass, cool night light, chunky silhouettes',
    palette: ['#1b2440', '#5b4a9e', '#9fc3ff', '#e8f1ff'],
    images: SEEDED.map(i => ({ ...i })),
    updatedAt: new Date(clock.nowMs() - 2 * 3_600_000).toISOString(),
  };
}

/** Which directions carried the seeded board: the arena now, the kit earlier, Elsheria not. */
export function markStyled(directions: Direction[]): Direction[] {
  return directions.map(d =>
    d.id === 'frozen-lake-arena'
      ? { ...d, styleRevision: 3 }
      : d.id === 'veil-gate-kit'
        ? { ...d, styleRevision: 2 }
        : d.id === 'elsheria-sheets'
          ? { ...d, styleOff: true }
          : d
  );
}

const stem = (path: string) =>
  path
    .slice(path.lastIndexOf('/') + 1)
    .replace(/\.[^.]+$/, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '_')
    .replace(/^_+|_+$/g, '') || 'object';

export function createStyleBoards(options: StyleBoardOptions) {
  const { fail } = options;
  const boards = new Map<string, StyleBoard>();
  if (options.state === 'board') {
    const board = frostveilStyleBoard();
    boards.set('frostveil', board);
    const dir = options.projectDir('frostveil');
    for (const image of board.images)
      options.files?.alias(`${dir}/${image.file}`, `${dir}/${image.source ?? ''}`);
  }
  const taken = new Set<string>();

  const board = (project: string): StyleBoard => boards.get(project) ?? { ...EMPTY_STYLE };

  /** Copies one entry into `style/`, or keeps a board image as it is. */
  function gather(project: string, before: StyleBoard, entries: string[]): StyleImage[] {
    const dir = options.projectDir(project);
    const out: StyleImage[] = [];
    for (const raw of entries) {
      const entry = raw.trim().replace(/\\/g, '/');
      if (!entry) continue;
      const kept = before.images.find(i => i.file === entry || i.source === entry);
      const image =
        kept ??
        (() => {
          if (entry.includes('..')) fail('invalid', `${entry} is not an image in this project.`);
          if (!/\.(png|jpe?g|webp)$/i.test(entry))
            fail('invalid', `${entry} is not a PNG, JPEG or WebP image.`);
          const outside = entry.startsWith('/');
          const ext = entry.slice(entry.lastIndexOf('.') + 1).toLowerCase();
          let name = `style/${stem(entry)}.${ext}`;
          for (let n = 2; taken.has(`${project}/${name}`); n += 1)
            name = `style/${stem(entry)}_${n}.${ext}`;
          taken.add(`${project}/${name}`);
          options.files?.alias(`${dir}/${name}`, outside ? entry : `${dir}/${entry}`);
          return { file: name, source: outside ? null : entry };
        })();
      if (out.some(i => i.file === image.file)) continue;
      out.push(image);
    }
    if (out.length > MAX_STYLE_IMAGES)
      fail('invalid', `The style board holds ${MAX_STYLE_IMAGES} images at most.`);
    return out;
  }

  function set(project: string, change: StyleBoardChange): StyleBoard {
    const before = board(project);
    const line =
      change.line == null ? before.line : change.line.split(/\s+/).filter(Boolean).join(' ');
    if (line.length > MAX_STYLE_LINE)
      fail('invalid', `Keep the style line to ${MAX_STYLE_LINE} characters or fewer.`);
    let palette = before.palette;
    if (change.palette != null) {
      palette = [];
      for (const raw of change.palette.filter(c => c.trim())) {
        const colour = normalizeColour(raw);
        if (!colour)
          return fail('invalid', `"${raw.trim()}" is not a colour. Write it as #rrggbb.`);
        if (!palette.includes(colour)) palette.push(colour);
      }
      if (palette.length > MAX_PALETTE)
        fail('invalid', `Keep the palette to ${MAX_PALETTE} colours.`);
    }
    const images = change.images == null ? before.images : gather(project, before, change.images);
    const same =
      line === before.line &&
      JSON.stringify(palette) === JSON.stringify(before.palette) &&
      JSON.stringify(images) === JSON.stringify(before.images);
    if (same) return before;
    const next: StyleBoard = {
      ...before,
      line,
      palette,
      images,
      revision: before.revision + 1,
      updatedAt: clock.now().toISOString(),
    };
    boards.set(project, next);
    return next;
  }

  /** The board a new job carries: none when turned off or empty. */
  const forJob = (project: string, useStyle: boolean | undefined): StyleBoard | null => {
    const current = board(project);
    return useStyle === false || isStyleEmpty(current) ? null : current;
  };

  return { board, set, forJob };
}
