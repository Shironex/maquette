/**
 * What a recipe changes against a library revision, in the core's plain
 * words, for mock mode's save to library sheet and What changed: the name,
 * the base clip, and each change and blank added, removed or edited
 * (matched by id). Which prop fills a prop blank is not counted, since
 * asset ids belong to each project.
 */
import type { RecipeBlank, RecipeFile, RecipeLayer } from '../client';
import { counted } from '@/lib/format';

const KIND: Record<RecipeLayer['kind'], string> = {
  reach: 'reach',
  nudge: 'nudge',
  'look-at': 'look at',
  wave: 'wave',
};

const layerWords = (l: RecipeLayer) => `the ${KIND[l.kind]} on ${l.bone} (${l.id})`;
const blankWords = (b: RecipeBlank) => `the ${b.kind} blank ${b.id} (${b.label})`;
const same = (a: unknown, b: unknown) => JSON.stringify(a ?? null) === JSON.stringify(b ?? null);

function layerEdits(a: RecipeLayer, b: RecipeLayer): string[] {
  const out: string[] = [];
  if (a.kind !== b.kind) out.push('kind');
  if (a.bone !== b.bone) out.push('bone');
  const keys = ['weight', 'start', 'end', 'target', 'rotation', 'offset', 'cycles', 'amount'];
  for (const key of keys) {
    const k = key as keyof RecipeLayer['settings'];
    if (!same(a.settings[k], b.settings[k])) out.push(key);
  }
  return out;
}

function blankEdits(a: RecipeBlank, b: RecipeBlank): string[] {
  const out: string[] = [];
  if (a.label !== b.label) out.push('label');
  if (a.kind !== b.kind) out.push('kind');
  if (!same(a.point, b.point)) out.push('point');
  return out;
}

/** Adds, edits and removes of entries matched by id. */
function listChanges<T extends { id: string }>(
  before: readonly T[],
  after: readonly T[],
  words: (item: T) => string,
  edits: (a: T, b: T) => string[]
): string[] {
  const out: string[] = [];
  for (const item of after) {
    const old = before.find(b => b.id === item.id);
    if (!old) out.push(`Adds ${words(item)}.`);
    else {
      const changed = edits(old, item);
      if (changed.length) out.push(`Edits ${words(item)}: ${changed.join(', ')}.`);
    }
  }
  for (const old of before)
    if (!after.some(a => a.id === old.id)) out.push(`Removes ${words(old)}.`);
  return out;
}

/**
 * What `after` changes against `before`, a line each; for no `before` (a
 * new library recipe), one line saying what it holds. Empty when both hold
 * the same name, base, blanks and changes.
 */
export function recipeChanges(before: RecipeFile | null, after: RecipeFile): string[] {
  if (!before)
    return [
      `A new library recipe on ${after.base.name} with ${counted(after.layers.length, 'change')} and ${counted(after.blanks.length, 'blank')}.`,
    ];
  const out: string[] = [];
  if (before.name !== after.name) out.push(`Renames it from ${before.name} to ${after.name}.`);
  const [b, a] = [before.base, after.base];
  if (b.name !== a.name || b.frames !== a.frames || b.hash !== a.hash)
    out.push(
      `Changes the base clip from ${b.name} (${b.frames} frames) to ${a.name} (${a.frames} frames).`
    );
  out.push(...listChanges(before.layers, after.layers, layerWords, layerEdits));
  const ids = (l: readonly RecipeLayer[]) => l.map(x => x.id);
  const sameSet =
    before.layers.length === after.layers.length &&
    before.layers.every(x => after.layers.some(y => y.id === x.id));
  if (sameSet && !same(ids(before.layers), ids(after.layers)))
    out.push('Lays the changes over the clip in another order.');
  out.push(...listChanges(before.blanks, after.blanks, blankWords, blankEdits));
  return out;
}
