/**
 * Animation recipes as the Motion view's Recipes tab shows them: a base
 * clip from the library with changes laid over it. Pure helpers; the store
 * is use-recipes-store.ts.
 */
import type { AnimationItem, RecipeSummary } from '@/ipc/client';
import { counted } from '@/lib/format';
import { NO_ANIMATION_FILTERS, filterClips } from './animations';

/** "Standing Idle", or "Sword Slash + 2 clips" for a recipe that joins two more. */
export const clipsWords = (base: string, joined: number) =>
  joined ? `${base} + ${counted(joined, 'clip')}` : base;

/**
 * "Standing Idle · 120 f · 2 changes · 1 open blank": the frames the whole
 * recipe runs, its clips joined and its time edits played.
 */
export function recipeLine(r: RecipeSummary): string {
  return [
    clipsWords(r.base, r.joined.length),
    `${r.length} f`,
    counted(r.layers, 'change'),
    r.open ? counted(r.open, 'open blank') : null,
  ]
    .filter(Boolean)
    .join(' · ');
}

/** The library clips a search keeps, for the base clip picker. */
export const pickableClips = (items: readonly AnimationItem[], query: string) =>
  filterClips(items, { ...NO_ANIMATION_FILTERS, query });
