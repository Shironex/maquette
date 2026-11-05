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
 * then nothing saves. Save reads the file first too, so an outside write
 * whose event has not arrived yet is never overwritten.
 */
import { create } from 'zustand';
import { commands, errorMessage, type RecipeDoc } from '@/ipc/client';
import { deepEqual } from '@/lib/deepEqual';
import { checkDraft, problemCount } from './recipe-check';
import { draftOf, sameDraft, type RecipeDraft } from './recipe-draft';
import type { RecipeEditing, RecipeEditorStore, RecipeLeave } from './recipe-editor.types';
import { mergeDrafts } from './recipe-merge';
import { useRecipesStore } from './use-recipes-store';

export type { RecipeLeave, RecipePick } from './recipe-editor.types';

export const SAVE_FAILED = 'The recipe was not saved';
export const FIX_FIRST = 'Fix what is marked first.';
export const ANSWER_FIRST = 'Choose Keep mine or Use theirs first.';
export const CHANGED_OUTSIDE = 'An agent changed this recipe; it shows as it is now.';
export const MERGED_OUTSIDE = 'An agent changed this recipe; its changes are merged with yours.';
export const GONE =
  'This recipe was removed outside. Copy anything you need; Save cannot bring it back.';

/** Whether the open recipe holds edits its file does not have. */
export const isDirty = (e: RecipeEditing | null) =>
  Boolean(e && !sameDraft(e.draft, draftOf(e.doc.recipe)));

/** Counts reads of the open recipe, so an older answer never replaces a newer one. */
let reads = 0;

export const useRecipeEditorStore = create<RecipeEditorStore>((set, get) => {
  const fresh = {
    saveError: null,
    outside: null,
    merged: null,
    gone: false,
    leaving: null,
  };
  const take = (project: string, doc: RecipeDoc, draft?: RecipeDraft) =>
    set({
      editing: { project, id: doc.id, doc, draft: draft ?? draftOf(doc.recipe) },
      ...fresh,
    });
  /** Lays `theirs` over the open recipe; false when it clashes with the edits and asks. */
  const absorb = (theirs: RecipeDoc) => {
    const editing = get().editing;
    if (!editing) return false;
    const base = draftOf(editing.doc.recipe);
    const next = draftOf(theirs.recipe);
    if (sameDraft(base, next)) {
      if (!deepEqual(editing.doc, theirs)) set({ editing: { ...editing, doc: theirs } });
      return true;
    }
    if (!isDirty(editing)) {
      set({ editing: { ...editing, doc: theirs, draft: next }, merged: CHANGED_OUTSIDE });
      return true;
    }
    const merge = mergeDrafts(base, editing.draft, next);
    if (merge.clashes.length) {
      set({ outside: { theirs, merge } });
      return false;
    }
    set({ editing: { ...editing, doc: theirs, draft: merge.mine }, merged: MERGED_OUTSIDE });
    return true;
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
        set({ editing: { ...editing, draft: draftOf(editing.doc.recipe) }, saveError: null });
    },

    save: async () => {
      const { editing, saving, outside } = get();
      if (!editing || saving) return false;
      if (!isDirty(editing)) return true;
      if (outside) {
        set({ saveError: ANSWER_FIRST });
        return false;
      }
      if (problemCount(checkDraft(editing.draft))) {
        set({ saveError: FIX_FIRST });
        return false;
      }
      set({ saving: true, saveError: null });
      try {
        // An outside write whose event has not come yet merges first, or asks.
        const now = await commands.recipeGet(editing.project, editing.id);
        if (!absorb(now)) {
          set({ saving: false });
          return false;
        }
        const ready = get().editing;
        if (!ready) return false;
        const sent = ready.draft;
        const blanksSame = deepEqual(sent.blanks, ready.doc.recipe.blanks);
        const doc = await commands.recipeUpdate(ready.project, ready.id, {
          name: null,
          layers: sent.layers,
          blanks: blanksSame ? null : sent.blanks,
        });
        const latest = get().editing;
        // Edits made while it saved stay over the answer.
        const draft = latest && !sameDraft(latest.draft, sent) ? latest.draft : undefined;
        if (latest?.id === ready.id) {
          set({ editing: { ...latest, doc, draft: draft ?? draftOf(doc.recipe) } });
        }
        set({ saving: false });
        void useRecipesStore.getState().load(ready.project);
        return true;
      } catch (error) {
        set({ saving: false, saveError: errorMessage(error) });
        return false;
      }
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
      try {
        const doc = await commands.recipeGet(project, editing.id);
        if (read === reads && get().editing?.id === editing.id) absorb(doc);
      } catch {
        if (read === reads && get().editing?.id === editing.id) set({ gone: true });
      }
    },

    keepMine: () => {
      const { editing, outside } = get();
      if (!editing || !outside) return;
      set({
        editing: { ...editing, doc: outside.theirs, draft: outside.merge.mine },
        outside: null,
      });
    },

    useTheirs: () => {
      const { editing, outside } = get();
      if (!editing || !outside) return;
      set({
        editing: { ...editing, doc: outside.theirs, draft: outside.merge.theirs },
        outside: null,
      });
    },

    saveForQuit: async () => {
      if (!isDirty(get().editing)) return null;
      return (await get().save()) ? null : (get().saveError ?? SAVE_FAILED);
    },
  };
});
