/**
 * Going through the inbox many models at a time: the layout (one by one or
 * the grouped table), the list filter, the bulk accept and discard, and the
 * discarded models with Restore. The selection itself is the inbox store's
 * `checked`, shared with the batch redo.
 */
import { create } from 'zustand';
import { commands, errorMessage, type AcceptResult, type DiscardedModel } from '@/ipc/client';
import { acceptTarget, goesTo, itemTitle } from './inbox';
import type { TriageFilter } from './triage';
import { useAppStore } from './use-app-store';
import { useInboxStore } from './use-inbox-store';
import { useLibraryStore } from './use-library-store';
import { usePrepareStore } from './use-prepare-store';

export type InboxLayout = 'one' | 'table';

/** What to do after a bulk accept: nothing, or add every accepted asset to the library. */
export interface AcceptAfter {
  library: boolean;
  /** Tags for every entry the library adds; empty keeps the entries' own. */
  tags: string[];
}

interface TriageStore {
  layout: InboxLayout;
  filter: TriageFilter;
  discarded: DiscardedModel[];
  /** A bulk accept or discard is running. */
  working: boolean;
  /** The discarded model Restore is running for. */
  restoring: string | null;
  setLayout: (layout: InboxLayout) => void;
  setFilter: (filter: TriageFilter) => void;
  loadDiscarded: () => Promise<void>;
  /** Accepts each model into the project it goes to; false when none went in. */
  acceptMany: (ids: readonly string[], after?: AcceptAfter) => Promise<boolean>;
  /** Moves each model aside into `inbox/.discarded`; false when none moved. */
  discardMany: (ids: readonly string[]) => Promise<boolean>;
  restore: (name: string) => Promise<void>;
}

const notify = (notice: Parameters<ReturnType<typeof useAppStore.getState>['notify']>[0]) =>
  useAppStore.getState().notify(notice);

/** "Ice crystals: Accept failed" lines, at most three, then how many more. */
function failures(lines: readonly string[]): string {
  const shown = lines.slice(0, 3).join(' ');
  return lines.length > 3 ? `${shown} And ${lines.length - 3} more.` : shown;
}

/** Adds each accepted asset to the library, one at a time; answers how many went in. */
async function addToLibrary(results: readonly AcceptResult[], tags: string[]): Promise<number> {
  let added = 0;
  for (const r of results) {
    try {
      const done = await commands.libraryAdd({
        project: r.project.id,
        asset: r.asset.id,
        name: null,
        tags: tags.length ? tags : null,
        into: null,
      });
      useAppStore.getState().setProject(done.project);
      added += 1;
    } catch {
      // Counted as not added; the notice says how many went in.
    }
  }
  if (added) useLibraryStore.getState().onChanged();
  return added;
}

export const useTriageStore = create<TriageStore>((set, get) => ({
  layout: 'one',
  filter: 'all',
  discarded: [],
  working: false,
  restoring: null,

  setLayout: layout => set({ layout }),
  setFilter: filter => set({ filter }),

  loadDiscarded: async () => {
    try {
      set({ discarded: await commands.inboxDiscarded() });
    } catch {
      // Recently discarded stays as it was; the inbox itself still works.
    }
  },

  acceptMany: async (ids, after) => {
    if (get().working) return false;
    set({ working: true });
    const inbox = useInboxStore.getState();
    const { workspace, setProject } = useAppStore.getState();
    const prepare = usePrepareStore.getState();
    const results: AcceptResult[] = [];
    const failed: string[] = [];
    try {
      for (const id of ids) {
        const item = inbox.snapshot?.items.find(i => i.id === id);
        if (!item) continue;
        if (item.state === 'failed') {
          failed.push(`${itemTitle(item)}: the file could not be read.`);
          continue;
        }
        const project = acceptTarget(item, workspace?.projects ?? [], workspace?.project ?? null);
        if (!project) {
          failed.push(`${itemTitle(item)}: create a project first.`);
          continue;
        }
        const goal = goesTo(item);
        const prep = prepare.drafts[id] ?? null;
        try {
          const result = await commands.inboxAccept({
            item: id,
            project: project.id,
            asset: goal && goal.project === project.id ? goal.asset : null,
            name: null,
            prep,
          });
          if (prep) prepare.setDraft(id, null);
          setProject(result.project);
          results.push(result);
        } catch (error) {
          failed.push(`${itemTitle(item)}: ${errorMessage(error)}`);
        }
      }
      const added = after?.library && results.length ? await addToLibrary(results, after.tags) : 0;
      const n = results.length;
      notify({
        status: failed.length ? (n ? 'info' : 'failed') : 'done',
        title: n
          ? `${n} ${n === 1 ? 'model' : 'models'} accepted${after?.library ? `, ${added} added to the library` : ''}`
          : 'Nothing was accepted',
        detail: failed.length ? failures(failed) : 'Each became a version of its asset.',
      });
      inbox.clearChecked();
      await inbox.load();
      return n > 0;
    } finally {
      set({ working: false });
    }
  },

  discardMany: async ids => {
    if (get().working) return false;
    set({ working: true });
    const inbox = useInboxStore.getState();
    let moved = 0;
    const failed: string[] = [];
    try {
      for (const id of ids) {
        const item = inbox.snapshot?.items.find(i => i.id === id);
        try {
          await commands.inboxDiscard(id);
          moved += 1;
        } catch (error) {
          failed.push(`${item ? itemTitle(item) : id}: ${errorMessage(error)}`);
        }
      }
      notify({
        status: failed.length ? (moved ? 'info' : 'failed') : 'done',
        title: `${moved} ${moved === 1 ? 'model' : 'models'} discarded`,
        detail: failed.length
          ? failures(failed)
          : 'Moved to inbox/.discarded in the workspace, not deleted. Restore puts one back.',
      });
      inbox.clearChecked();
      await Promise.all([inbox.load(), get().loadDiscarded()]);
      return moved > 0;
    } finally {
      set({ working: false });
    }
  },

  restore: async name => {
    if (get().restoring) return;
    set({ restoring: name });
    try {
      const back = await commands.inboxRestore(name);
      notify({
        status: 'done',
        title: `${back.id} is back in the Inbox`,
        detail: 'Its checks run again. Accept goes to the asset it was going to.',
      });
      await Promise.all([useInboxStore.getState().load(), get().loadDiscarded()]);
      useInboxStore.getState().select(back.id);
    } catch (error) {
      notify({ status: 'failed', title: 'Could not restore it', detail: errorMessage(error) });
    } finally {
      set({ restoring: null });
    }
  },
}));
