/**
 * A recipe changed outside (an agent's update_recipe) while the editor holds
 * unsaved edits: the three versions are merged change by change and blank by
 * blank, by id, and field by field for the clips and the mirror. What only
 * one side touched comes through; what both changed differently is a clash,
 * answered Keep mine or Use theirs. An id both sides added with different
 * entries is two additions, never a clash: the person's gets a fresh id (and
 * the changes aiming at a renamed blank follow).
 *
 * The clips and the time edits are one field: a time edit sits on a clip by
 * its place, so clips moved on one side and time edits changed on the other
 * could land a time edit on the wrong clip. Both sides touching them clash.
 */
import { deepEqual } from '@/lib/deepEqual';
import { rekeep } from './recipe-clips';
import { freeId, type RecipeDraft } from './recipe-draft';

/** The clash words of the clips field and of the mirror. */
export const CLIPS_FIELD = 'the clips and time edits';
export const MIRROR_FIELD = 'the mirror';

type ClipsField = Pick<RecipeDraft, 'clips' | 'kept' | 'time'>;

const clipsField = (d: RecipeDraft): ClipsField => ({ clips: d.clips, kept: d.kept, time: d.time });

/**
 * One field of three versions: theirs when mine did not touch it, mine
 * otherwise, theirs on the `theirs` side of a clash.
 */
function mergeField<T>(base: T, mine: T, theirs: T) {
  const mineChanged = !deepEqual(base, mine);
  const theirsChanged = !deepEqual(base, theirs);
  if (!mineChanged) return { mine: theirs, theirs, clash: false };
  const clash = theirsChanged && !deepEqual(mine, theirs);
  return { mine, theirs: clash ? theirs : mine, clash };
}

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
  /** The person's entries renamed because the outside side added the same id. */
  renamed: Renamed;
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

/** `mine` with the ids both sides added kept apart, and the new id of each entry renamed. */
function apart(
  base: RecipeDraft,
  mine: RecipeDraft,
  theirs: RecipeDraft
): { draft: RecipeDraft; renamed: Renamed } {
  const blankIds = renames(base.blanks, mine.blanks, theirs.blanks, b => b.kind);
  const layerIds = renames(base.layers, mine.layers, theirs.layers, () => 'l');
  const renamed = {
    layer: Object.fromEntries(layerIds),
    blank: Object.fromEntries(blankIds),
  };
  if (!blankIds.size && !layerIds.size) return { draft: mine, renamed };
  const draft = {
    ...mine,
    blanks: mine.blanks.map(b => ({ ...b, id: blankIds.get(b.id) ?? b.id })),
    layers: mine.layers.map(l => {
      const target = l.settings.target;
      const aim = target ? (blankIds.get(target) ?? target) : target;
      return { ...l, id: layerIds.get(l.id) ?? l.id, settings: { ...l.settings, target: aim } };
    }),
  };
  return { draft, renamed };
}

/** The new id of each of the person's entries the merge renamed, by kind. */
export interface Renamed {
  layer: Record<string, string>;
  blank: Record<string, string>;
}

/** Lays the outside change from `base` to `theirs` over the person's `mine`. */
export function mergeDrafts(
  base: RecipeDraft,
  ownDraft: RecipeDraft,
  theirs: RecipeDraft
): DraftMerge {
  const { draft: mine, renamed } = apart(base, ownDraft, theirs);
  const layers = mergeList(base.layers, mine.layers, theirs.layers);
  const blanks = mergeList(base.blanks, mine.blanks, theirs.blanks);
  const clips = mergeField(clipsField(base), clipsField(mine), clipsField(theirs));
  const mirror = mergeField(base.mirror, mine.mirror, theirs.mirror);
  // Mine counts its kept clips against the file as read; the merge lands on theirs.
  const onTheirs = (c: ClipsField): ClipsField =>
    c === clipsField(theirs) || deepEqual(c, clipsField(theirs))
      ? c
      : { ...c, kept: rekeep(c.clips, c.kept, base.clips, theirs.clips) };
  return {
    mine: {
      layers: layers.mine,
      blanks: blanks.mine,
      ...onTheirs(clips.mine),
      mirror: mirror.mine,
    },
    theirs: {
      layers: layers.theirs,
      blanks: blanks.theirs,
      ...onTheirs(clips.theirs),
      mirror: mirror.theirs,
    },
    clashes: [
      ...layers.clashes.map(id => `change ${id}`),
      ...blanks.clashes.map(id => `blank ${id}`),
      ...(clips.clash ? [CLIPS_FIELD] : []),
      ...(mirror.clash ? [MIRROR_FIELD] : []),
    ],
    renamed,
  };
}
