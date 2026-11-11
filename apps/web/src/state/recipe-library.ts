/**
 * The recipe library as the Animations library and the Motion view's
 * Recipes tab show it: recipes kept outside any project, each with its
 * revisions. Pure helpers; the stores are use-recipe-library-store.ts,
 * use-recipe-save-store.ts and use-recipe-compare-store.ts.
 */
import type {
  GripFrom,
  LibraryGripRevision,
  LibraryGripSummary,
  LibraryRecipeRevision,
  LibraryRecipeSummary,
  RecipeFrom,
  RecipeUsed,
} from '@/ipc/client';
import { counted } from '@/lib/format';

/** "Standing Idle · 120 f · 3 changes · r3". */
export const libraryLine = (s: LibraryRecipeSummary) =>
  [s.base, `${s.frames} f`, counted(s.layers, 'change'), `r${s.latest}`].join(' · ');

/** "r3 · both hands · 33 bones": a library grip in one line. */
export const gripLine = (g: LibraryGripSummary) =>
  [`r${g.latest}`, g.handsWords, counted(Object.keys(g.pose).length, 'bone')].join(' · ');

/** "Saved from Frostveil / reach-for-the-lantern", or "Imported from wave.recipe.json". */
export function revisionSource(r: LibraryRecipeRevision, projectName: (id: string) => string) {
  if (r.imported) return `Imported from ${r.imported}`;
  if (r.project) return `Saved from ${projectName(r.project)} / ${r.recipe ?? 'a recipe'}`;
  return 'Saved';
}

/** "Saved from Frostveil / scythe-charge-held, frame 30", or "Saved". */
export function gripRevisionSource(r: LibraryGripRevision, projectName: (id: string) => string) {
  if (!r.project) return 'Saved';
  const frame = r.frame === null ? '' : `, frame ${r.frame}`;
  return `Saved from ${projectName(r.project)} / ${r.recipe ?? 'a recipe'}${frame}`;
}

/** The revisions, newest first. */
export const newestFirst = <R extends { revision: number }>(revisions: readonly R[]) =>
  [...revisions].sort((a, b) => b.revision - a.revision);

/** What the library says about a project recipe that came from one of its recipes. */
export type LibraryNote = { kind: 'newer'; entry: string; latest: number } | { kind: 'gone' };

/**
 * "Library has rN" when the library recipe a project recipe came from has
 * a newer revision, "gone" when it was removed, none otherwise or while
 * the library is not read yet.
 */
export function libraryNote(
  from: RecipeFrom | null,
  list: readonly LibraryRecipeSummary[] | null
): LibraryNote | null {
  if (!from || !list) return null;
  // Same id and uid: an id taken again after a removal is another recipe.
  const entry = list.find(e => e.id === from.entry && e.uid === from.uid);
  if (!entry) return { kind: 'gone' };
  return entry.latest > from.revision
    ? { kind: 'newer', entry: entry.id, latest: entry.latest }
    : null;
}

/** What the grip library says about a grip change that was copied from one of its grips. */
export type GripNote =
  { kind: 'newer'; entry: string; name: string; latest: number } | { kind: 'gone' };

/**
 * "Update to rN" when the library grip a grip change was copied from has a
 * newer revision, "gone" when it was removed, none otherwise or while the
 * library is not read yet. A link without a uid (written by hand) goes by
 * the id alone.
 */
export function gripNote(
  from: GripFrom | null | undefined,
  list: readonly LibraryGripSummary[] | null
): GripNote | null {
  if (!from || !list) return null;
  const entry = list.find(e => e.id === from.entry && (!from.uid || e.uid === from.uid));
  if (!entry) return { kind: 'gone' };
  return entry.latest > from.revision
    ? { kind: 'newer', entry: entry.id, name: entry.name, latest: entry.latest }
    : null;
}

/** "Using r3 made Reach for the lantern" and what to know, for a notice. */
export function usedNotice(used: RecipeUsed, revision: number) {
  return {
    title: `Made ${used.doc.recipe.name} from r${revision}`,
    detail: used.notes.length ? used.notes.join(' ') : undefined,
  };
}
