/** Pure helpers for the Lab's style board: limits, colours, summaries and the per direction chip. */
import type { Direction, StyleBoard } from '@/ipc/client';

/** The most images the board holds; the core says the same. */
export const MAX_STYLE_IMAGES = 4;
/** The longest style line; the core says the same. */
export const MAX_STYLE_LINE = 200;
/** The most palette colours; the core says the same. */
export const MAX_PALETTE = 6;

export const EMPTY_STYLE: StyleBoard = {
  version: 1,
  revision: 0,
  line: '',
  palette: [],
  images: [],
  updatedAt: null,
};

/** True when the board has nothing to carry, so jobs carry nothing either way. */
export function isStyleEmpty(board: Pick<StyleBoard, 'line' | 'palette' | 'images'>): boolean {
  return board.line.trim() === '' && board.palette.length === 0 && board.images.length === 0;
}

/** `#rrggbb` in lowercase from `#rgb`, `rrggbb` and the like, or null; as the core reads it. */
export function normalizeColour(text: string): string | null {
  const hex = text.trim().replace(/^#/, '').toLowerCase();
  if (!/^[0-9a-f]+$/.test(hex)) return null;
  if (hex.length === 6) return `#${hex}`;
  if (hex.length === 3) return `#${[...hex].map(c => c + c).join('')}`;
  return null;
}

/** Adds typed colours (spaces or commas between them), without repeats, at most six. */
export function addColours(current: readonly string[], text: string): string[] | null {
  const parts = text.split(/[\s,;]+/).filter(Boolean);
  const out = [...current];
  for (const part of parts) {
    const colour = normalizeColour(part);
    if (!colour) return null;
    if (!out.includes(colour) && out.length < MAX_PALETTE) out.push(colour);
  }
  return out;
}

/** "3 images · 4 colours", what the board holds besides its line. */
export function styleCounts(board: Pick<StyleBoard, 'palette' | 'images'>): string {
  const parts: string[] = [];
  const n = board.images.length;
  const c = board.palette.length;
  if (n) parts.push(`${n} ${n === 1 ? 'image' : 'images'}`);
  if (c) parts.push(`${c} ${c === 1 ? 'colour' : 'colours'}`);
  return parts.join(' · ');
}

export interface StyleChip {
  label: string;
  tip: string;
  on: boolean;
}

/**
 * The chip a direction shows: the board revision its latest batch carried,
 * or that it was made without the board while the project has one.
 */
export function styleChip(
  direction: Pick<Direction, 'styleOff' | 'styleRevision'>,
  board: StyleBoard
): StyleChip | null {
  if (direction.styleRevision !== null) {
    const now = direction.styleRevision === board.revision;
    return {
      label: `Style r${direction.styleRevision}`,
      tip: now
        ? 'Made with the project style as it is now'
        : `Made with the project style at revision ${direction.styleRevision}; the board is at ${board.revision} now`,
      on: true,
    };
  }
  if (direction.styleOff && !isStyleEmpty(board)) {
    return { label: 'Own style', tip: 'Made without the project style', on: false };
  }
  return null;
}
