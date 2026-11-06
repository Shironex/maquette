/**
 * The Recipes tab's editor: one recipe open, its changes and blanks as the
 * person edits them, and Save through `recipe_update` with the lists whole.
 *
 * The edits live here, not in the view, so leaving the tab or the Motion
 * view keeps them. Leaving the recipe itself (Back, another recipe, another
 * project, quitting) with unsaved edits asks first: save, leave without
 * saving, or stay. Nothing is dropped without that answer, as the planning
 * canvas does it.
 *
 * Outside edits (an agent's update_recipe, which sends `recipe-changed`) are
 * read again: with no unsaved edits they simply show; with some, they are
 * merged change by change and blank by blank, and only when both sides
 * changed the same entry does `outside` ask Keep mine or Use theirs. Until
 * then nothing saves. Save reads the file first and merges what came in, then
 * sends the version (a hash of the file) it built on: the core refuses the
 * write when the file moved since in any way (code `moved`), so a write
 * landing in between is never overwritten. The editor then reads and merges again (asking only on a real
 * clash) and the person saves once more. Only the lists that changed are sent.
 */
import { create } from 'zustand';
import { commands, errorMessage, type RecipeDoc } from '@/ipc/client';
import { useAppStore } from './use-app-store';
import { deepEqual } from '@/lib/deepEqual';
import { bakeModelOf } from './recipe-bake';
import { checkDraft, problemCount } from './recipe-check';
import { draftOf, sameDraft } from './recipe-draft';
import type {
  RecipeEditing,
  RecipeEditorStore,
  RecipeLeave,
  RecipePick,
} from './recipe-editor.types';
import { mergeDrafts, type Renamed } from './recipe-merge';
import { useMotionStore } from './use-motion-store';
import { useRecipesStore } from './use-recipes-store';

export type { RecipeLeave, RecipePick } from './recipe-editor.types';

export const SAVE_FAILED = 'The recipe was not saved';
export const FIX_FIRST = 'Fix what is marked first.';
export const ANSWER_FIRST = 'Choose Keep mine or Use theirs first.';
export const CHANGED_OUTSIDE = 'An agent changed this recipe; it shows as it is now.';
export const MERGED_OUTSIDE = 'An agent changed this recipe; its changes are merged with yours.';
export const MOVED =
  'The recipe changed while it saved, so nothing was written. Its changes are merged with yours: save again.';
export const GONE =
  'This recipe was removed outside. Copy anything you need; Save cannot bring it back.';

/**
 * The gone props the next save empties for good: only those still prop
 * blanks of the draft, so a blank removed or turned into a point since
 * Leave open is never sent in `clear`.
 */
export const clearsOf = (e: RecipeEditing) =>
  e.cleared.filter(id => e.draft.blanks.some(b => b.id === id && b.kind === 'prop'));

/** Whether the open recipe holds edits its file does not have. */
export const isDirty = (e: RecipeEditing | null) =>
  Boolean(e && (clearsOf(e).length > 0 || !sameDraft(e.draft, draftOf(e.doc.recipe))));

/** The error code the core answers a write with when the file moved since it was read. */
const MOVED_CODE = 'moved';
/** The error code the core answers a read of a file that is not there with. */
const NOT_FOUND_CODE = 'not-found';
const codeOf = (error: unknown) =>
  typeof error === 'object' && error !== null && 'code' in error ? String(error.code) : null;

/** Counts reads of the open recipe, so an older answer never replaces a newer one. */
let reads = 0;
/** The save running now, so a second Save, or quitting, waits for it. */
let running: Promise<boolean> | null = null;

/** `editing` on `doc`, keeping only the clears its gone props and the draft still need. */
const onDoc = (editing: RecipeEditing, doc: RecipeDoc, draft: RecipeEditing['draft']) => {
  const next = { ...editing, doc, draft };
  return { ...next, cleared: clearsOf(next).filter(id => doc.gone.some(g => g.blank === id)) };
};

/** `picked` after a merge renamed the person's entries, so its settings stay open on the same one. */
const follow = (picked: RecipePick, renamed: Renamed): RecipePick =>
  picked ? { ...picked, id: renamed[picked.what][picked.id] ?? picked.id } : picked;

export const useRecipeEditorStore = create<RecipeEditorStore>((set, get) => {
  const fresh = {
    saveError: null,
    outside: null,
    merged: null,
    gone: false,
    unreadable: false,
    leaving: null,
  };
  const take = (project: string, doc: RecipeDoc) => {
    const open = useAppStore.getState().workspace?.project;
    const assets = open?.id === project ? (open.assets ?? []) : [];
    set({
      editing: { project, id: doc.id, doc, draft: draftOf(doc.recipe), cleared: [], assets },
      ...fresh,
    });
  };
  /** Lays `theirs` over the open recipe; false when it clashes with the edits and asks. */
  const absorb = (theirs: RecipeDoc) => {
    const editing = get().editing;
    if (!editing) return false;
    const base = draftOf(editing.doc.recipe);
    const next = draftOf(theirs.recipe);
    // A clash asked about an older version is stale now: this read answers it afresh.
    if (sameDraft(base, next)) {
      if (!deepEqual(editing.doc, theirs))
        set({ editing: onDoc(editing, theirs, editing.draft), outside: null });
      else set({ outside: null });
      return true;
    }
    if (!isDirty(editing)) {
      set({ editing: onDoc(editing, theirs, next), merged: CHANGED_OUTSIDE, outside: null });
      return true;
    }
    const merge = mergeDrafts(base, editing.draft, next);
    if (merge.clashes.length) {
      set({ outside: { theirs, merge } });
      return false;
    }
    set({
      editing: onDoc(editing, theirs, merge.mine),
      merged: MERGED_OUTSIDE,
      outside: null,
      picked: follow(get().picked, merge.renamed),
    });
    return true;
  };
  const saveNow = async () => {
    const { editing, outside } = get();
    if (!editing) return false;
    if (!isDirty(editing)) {
      // Nothing left to write: whatever came in is what the file holds.
      if (!outside) set({ merged: null });
      return true;
    }
    if (outside) {
      set({ saveError: ANSWER_FIRST });
      return false;
    }
    const { view, asset } = useMotionStore.getState();
    if (problemCount(checkDraft(editing.draft, bakeModelOf(view, asset)))) {
      set({ saveError: FIX_FIRST });
      return false;
    }
    // A read that started before this save must not land after it with the old file.
    reads += 1;
    set({ saving: true, saveError: null });
    try {
      // An outside write whose event has not come yet merges first, or asks.
      if (!absorb(await commands.recipeGet(editing.project, editing.id))) {
        set({ saving: false });
        return false;
      }
      const ready = get().editing;
      if (!ready) return false;
      const sent = ready.draft;
      const stored = ready.doc.recipe;
      const cleared = clearsOf(ready);
      const doc = await commands.recipeUpdate(ready.project, ready.id, {
        name: null,
        layers: deepEqual(sent.layers, stored.layers) ? null : sent.layers,
        blanks: deepEqual(sent.blanks, stored.blanks) && !cleared.length ? null : sent.blanks,
        expected: ready.doc.version,
        clear: cleared,
      });
      // Nor one that started while it saved: it may have read the file before the write.
      reads += 1;
      const latest = get().editing;
      if (latest?.id === ready.id) {
        // Edits made while it saved stay over the answer.
        const draft = sameDraft(latest.draft, sent) ? draftOf(doc.recipe) : latest.draft;
        const left = latest.cleared.filter(id => !cleared.includes(id));
        // The merged edits are written now, so the note about them is done.
        set({ editing: onDoc({ ...latest, cleared: left }, doc, draft), merged: null });
      }
      set({ saving: false });
      void useRecipesStore.getState().load(ready.project);
      return true;
    } catch (error) {
      reads += 1;
      if (codeOf(error) !== MOVED_CODE) {
        set({ saving: false, saveError: errorMessage(error) });
        return false;
      }
      // Written meanwhile: merge it in, asking only on a clash, and let the person save again.
      try {
        const now = await commands.recipeGet(editing.project, editing.id);
        const merged = get().editing?.id === editing.id && absorb(now);
        set({ saving: false, saveError: merged ? MOVED : null });
      } catch (reread) {
        set({ saving: false, saveError: errorMessage(reread) });
      }
      return false;
    }
  };
  const go = async (leave: RecipeLeave) => {
    const editing = get().editing;
    set({ editing: null, picked: null, ...fresh });
    if (leave.to === 'recipe' && editing) await get().open(editing.project, leave.id);
  };

  return {
    editing: null,
    opening: null,
    openError: null,
    saving: false,
    picked: null,
    ...fresh,

    open: async (project, id) => {
      const editing = get().editing;
      if (editing?.project === project && editing.id === id) return;
      if (isDirty(editing)) {
        set({ leaving: { to: 'recipe', id } });
        return;
      }
      const read = ++reads;
      set({ opening: id, openError: null });
      try {
        const doc = await commands.recipeGet(project, id);
        if (read !== reads) return;
        take(project, doc);
        set({ opening: null, picked: null });
      } catch (error) {
        if (read === reads) set({ opening: null, openError: errorMessage(error) });
      }
    },

    leave: async leave => {
      if (isDirty(get().editing)) {
        set({ leaving: leave });
        return false;
      }
      await go(leave);
      return true;
    },

    edit: change => {
      const editing = get().editing;
      if (!editing) return;
      set({ editing: { ...editing, draft: change(editing.draft) }, saveError: null });
    },

    pick: picked => set({ picked }),

    discard: () => {
      const editing = get().editing;
      if (editing)
        set({
          editing: { ...editing, draft: draftOf(editing.doc.recipe), cleared: [] },
          saveError: null,
          merged: null,
        });
    },

    clearGone: (id, clear) => {
      const editing = get().editing;
      if (!editing || !editing.doc.gone.some(g => g.blank === id)) return;
      const others = editing.cleared.filter(c => c !== id);
      set({ editing: { ...editing, cleared: clear ? [...others, id] : others }, saveError: null });
    },

    save: () => {
      running ??= saveNow().finally(() => {
        running = null;
      });
      return running;
    },

    saveAndLeave: async () => {
      const leave = get().leaving;
      if (!leave || !(await get().save())) return;
      await go(leave);
    },

    leaveAnyway: async () => {
      const leave = get().leaving;
      if (leave) await go(leave);
    },

    stay: () => set({ leaving: null }),

    onChanged: async project => {
      const editing = get().editing;
      if (!editing || editing.project !== project) return;
      const read = ++reads;
      const current = () => read === reads && get().editing?.id === editing.id;
      try {
        const doc = await commands.recipeGet(project, editing.id);
        if (!current()) return;
        set({ gone: false, unreadable: false });
        absorb(doc);
      } catch (error) {
        // Only a file that is not there is gone; a half written or broken
        // file, or a one-off IPC error, keeps the edits and waits for a good read.
        if (current())
          set(
            codeOf(error) === NOT_FOUND_CODE
              ? { gone: true, unreadable: false }
              : { gone: false, unreadable: true }
          );
      }
    },

    keepMine: () => {
      const { editing, outside } = get();
      if (!editing || !outside) return;
      set({
        editing: onDoc(editing, outside.theirs, outside.merge.mine),
        outside: null,
        picked: follow(get().picked, outside.merge.renamed),
      });
    },

    useTheirs: () => {
      const { editing, outside } = get();
      if (!editing || !outside) return;
      set({
        editing: onDoc(editing, outside.theirs, outside.merge.theirs),
        outside: null,
        picked: follow(get().picked, outside.merge.renamed),
      });
    },

    saveForQuit: async () => {
      // A Save already running is waited for, never reported as not saved.
      if (running) await running;
      if (!isDirty(get().editing)) return null;
      return (await get().save()) ? null : (get().saveError ?? SAVE_FAILED);
    },
  };
});
