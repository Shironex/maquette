/**
 * Draft marks in the editor: what agents changed in the open recipe since a
 * person saved it. Their writes are in the recipe at once and bake as they
 * are; the editor shows them in the agent's blue until a person saves,
 * which keeps them as theirs and clears the marks.
 */
import type { RecipeDrafts, RecipeLayer } from '@/ipc/client';
import { formatAgo } from '@/lib/format';

/** The legend under the banner. */
export const DRAFTS_LEGEND = 'Blue: drafted by an agent, until you save.';

const plural = (n: number, one: string) => `${n} ${one}${n === 1 ? '' : 's'}`;

/** Whether the change `id` is drafted. */
export const draftedLayer = (drafts: RecipeDrafts | null | undefined, id: string) =>
  Boolean(drafts?.layers?.includes(id));

/** Whether the key at `clip`'s own `frame` on the change `layer` is drafted. */
export const draftedKey = (
  drafts: RecipeDrafts | null | undefined,
  layer: string,
  clip: number,
  frame: number
) => Boolean(drafts?.keys?.some(k => k.layer === layer && k.clip === clip && k.frame === frame));

/** "2 of 6 keys drafted" for a pose key change, or null when none is. */
export function keysDrafted(drafts: RecipeDrafts | null | undefined, layer: RecipeLayer) {
  const all = layer.keys ?? [];
  const n = all.filter(k => draftedKey(drafts, layer.id, k.clip ?? 0, k.frame)).length;
  return n ? `${n} of ${plural(all.length, 'key')} drafted` : null;
}

/** What the marks cover, in words: "2 keys, 1 change and the time edits". */
function covered(d: RecipeDrafts): string {
  const parts = [
    d.keys?.length ? plural(d.keys.length, 'key') : null,
    d.layers?.length ? plural(d.layers.length, 'change') : null,
    d.clips ? 'the clips' : null,
    d.time ? 'the time edits' : null,
    d.blanks ? 'the blanks' : null,
    d.mirror ? 'the mirror' : null,
  ].filter((p): p is string => Boolean(p));
  if (parts.length < 2) return parts[0] ?? 'the recipe';
  return `${parts.slice(0, -1).join(', ')} and ${parts.at(-1)}`;
}

/** The banner's title and detail for `drafts`, at `nowMs`. */
export function draftsBanner(drafts: RecipeDrafts, nowMs: number) {
  const when = Date.parse(drafts.at ?? '');
  const ago = Number.isNaN(when) ? '' : ` ${formatAgo(when, nowMs)}`;
  const what = drafts.made ? 'made this recipe' : 'changed this recipe';
  return {
    title: `${drafts.by || 'An agent'} ${what}${ago}`,
    detail: `Drafted: ${covered(drafts)}. They are in the recipe now and bake as they are. Saving keeps them as yours.`,
  };
}
