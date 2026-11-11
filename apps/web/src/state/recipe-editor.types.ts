import type { Asset, RecipeDoc } from '@/ipc/client';
import type { RecipeDraft } from './recipe-draft';
import type { DraftMerge } from './recipe-merge';

/** Where the person was going when unsaved edits stopped them. */
export type RecipeLeave =
  { to: 'list' } | { to: 'recipe'; id: string } | { to: 'project'; project: string };

/** The recipe open in the editor: the file as last read (the base) and the edits over it. */
export interface RecipeEditing {
  project: string;
  id: string;
  doc: RecipeDoc;
  draft: RecipeDraft;
  /** Prop blanks whose gone asset the person chose to empty for good on the next save. */
  cleared: string[];
  /** The project's assets when it opened, for when another project is open meanwhile. */
  assets: Asset[];
}

/** An outside change that clashes with unsaved edits, waiting for its answer. */
export interface RecipeOutside {
  theirs: RecipeDoc;
  merge: DraftMerge;
}

/**
 * The change, blank or base edit whose settings show: a base edit's id is
 * the clip's or time edit's index, or empty for the clips and the mirror.
 * `grips` is Add a grip's picker, its id the library grip picked in it.
 */
export type RecipePick = {
  what: 'layer' | 'blank' | 'clips' | 'trim' | 'time' | 'mirror' | 'grips';
  id: string;
} | null;

export interface RecipeEditorStore {
  editing: RecipeEditing | null;
  opening: string | null;
  openError: string | null;
  saving: boolean;
  saveError: string | null;
  outside: RecipeOutside | null;
  /** A line saying an outside change came in, merged or shown. */
  merged: string | null;
  /** The file went away outside while it was open (the read answered not found). */
  gone: boolean;
  /**
   * The last outside read failed for another reason: a file half written or
   * not valid JSON, or a one-off IPC error. The edits are kept; the next good
   * read clears it.
   */
  unreadable: boolean;
  /** Unsaved edits stopped a leave; the question is open while set. */
  leaving: RecipeLeave | null;
  picked: RecipePick;

  /** Opens a recipe of `project`, asking first when the open one has unsaved edits. */
  open: (project: string, id: string) => Promise<void>;
  /** Goes to `leave` now, or asks first when there are unsaved edits; true when it went. */
  leave: (leave: RecipeLeave) => Promise<boolean>;
  edit: (change: (draft: RecipeDraft) => RecipeDraft) => void;
  pick: (pick: RecipePick) => void;
  /** Drops the unsaved edits. */
  discard: () => void;
  /** Empties the gone prop of blank `id` for good on the next save, or keeps it (`clear` false). */
  clearGone: (id: string, clear: boolean) => void;
  /** Saves the edits; false, with `saveError` or `outside` set, when it did not. */
  save: () => Promise<boolean>;
  /** After a stopped leave: save, then go on when it worked. */
  saveAndLeave: () => Promise<void>;
  /** After a stopped leave: drop the edits and go on. */
  leaveAnyway: () => Promise<void>;
  stay: () => void;
  /** An agent changed one of `project`'s recipes. */
  onChanged: (project: string) => Promise<void>;
  keepMine: () => void;
  useTheirs: () => void;
  /** Saves the edits for quitting; why it could not, or null. */
  saveForQuit: () => Promise<string | null>;
}
