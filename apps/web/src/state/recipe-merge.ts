/**
 * A recipe changed outside (an agent's update_recipe) while the editor holds
 * unsaved edits: the three versions are merged change by change and blank by
 * blank, by id. What only one side touched comes through; what both changed
 * differently is a clash, answered Keep mine or Use theirs. An id both
 * sides added with different entries is two additions, never a clash: the
 * person's gets a fresh id (and the changes aiming at a renamed blank follow).
 */
import { deepEqual } from '@/lib/deepEqual';
import { freeId, type RecipeDraft } from './recipe-draft';

interface Keyed {
  id: string;
}

interface ListMerge<T> {
  /** With mine on each clash. */
  mine: T[];
  /** With theirs on each clash. */
  theirs: T[];
  clashes: string[];
}

function mergeList<T extends Keyed>(base: T[], mine: T[], theirs: T[]): ListMerge<T> {
  const find = (list: T[], id: string) => list.find(item => item.id === id);
  const order = [...mine.map(m => m.id)];
  for (const t of theirs) if (!order.includes(t.id) && !find(base, t.id)) order.push(t.id);
  // Ones mine removed but theirs changed are clashes too: keep their place at the end.
  for (const t of theirs) if (!order.includes(t.id)) order.push(t.id);
  const keepMine: T[] = [];
  const keepTheirs: T[] = [];
  const clashes: string[] = [];
  for (const id of order) {
    const b = find(base, id);
    const m = find(mine, id);
    const t = find(theirs, id);
    const mineChanged = !deepEqual(b, m);
    const theirsChanged = !deepEqual(b, t);
    let left = m;
    let right = m;
    if (!mineChanged) left = right = t;
    else if (theirsChanged && !deepEqual(m, t)) {
      clashes.push(id);
      right = t;
    }
    if (left) keepMine.push(left);
    if (right) keepTheirs.push(right);
  }
  return { mine: keepMine, theirs: keepTheirs, clashes };
}

export interface DraftMerge {
  /** Everything merged, mine on each clash. */
  mine: RecipeDraft;
  /** Everything merged, theirs on each clash. */
  theirs: RecipeDraft;
  /** "change l-2", "blank lantern". */
  clashes: string[];
}

/** Fresh ids for the entries of `mine` that `theirs` added under the same id with something else. */
function renames<T extends Keyed>(
  base: T[],
  mine: T[],
  theirs: T[],
  prefix: (item: T) => string
): Map<string, string> {
  const taken = [...base, ...mine, ...theirs].map(i => i.id);
  const out = new Map<string, string>();
  for (const m of mine) {
    const t = theirs.find(i => i.id === m.id);
    if (t && !base.some(i => i.id === m.id) && !deepEqual(m, t))
      out.set(m.id, freeId(prefix(m), taken));
  }
  return out;
}

/** `mine` with the ids both sides added kept apart. */
function apart(base: RecipeDraft, mine: RecipeDraft, theirs: RecipeDraft): RecipeDraft {
  const blankIds = renames(base.blanks, mine.blanks, theirs.blanks, b => b.kind);
  const layerIds = renames(base.layers, mine.layers, theirs.layers, () => 'l');
  if (!blankIds.size && !layerIds.size) return mine;
  return {
    blanks: mine.blanks.map(b => ({ ...b, id: blankIds.get(b.id) ?? b.id })),
    layers: mine.layers.map(l => {
      const target = l.settings.target;
      const aim = target ? (blankIds.get(target) ?? target) : target;
      return { ...l, id: layerIds.get(l.id) ?? l.id, settings: { ...l.settings, target: aim } };
    }),
  };
}

/** Lays the outside change from `base` to `theirs` over the person's `mine`. */
export function mergeDrafts(
  base: RecipeDraft,
  ownDraft: RecipeDraft,
  theirs: RecipeDraft
): DraftMerge {
  const mine = apart(base, ownDraft, theirs);
  const layers = mergeList(base.layers, mine.layers, theirs.layers);
  const blanks = mergeList(base.blanks, mine.blanks, theirs.blanks);
  return {
    mine: { layers: layers.mine, blanks: blanks.mine },
    theirs: { layers: layers.theirs, blanks: blanks.theirs },
    clashes: [
      ...layers.clashes.map(id => `change ${id}`),
      ...blanks.clashes.map(id => `blank ${id}`),
    ],
  };
}
