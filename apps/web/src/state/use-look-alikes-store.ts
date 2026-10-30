/**
 * The Library's look-alikes: pairs of models that look the same under two
 * names, found by picture the way the Inbox matches downloads, and pairs of
 * different models under one name. Merging and every choice is a person's;
 * a merge can be undone from the list of recent merges.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type LookAlikePair,
  type LookAlikeSettings,
  type LookAlikeVerdict,
  type LookAlikes,
} from '@/ipc/client';
import { useAppStore } from './use-app-store';
import { useLibraryStore } from './use-library-store';
import { renameEntry, restoreEntries } from './library-actions';

/** One pair's key, the same whichever way round. */
export const pairKey = (p: { a: string; b: string }) => [p.a, p.b].sort().join('|');

interface LookAlikesStore {
  report: LookAlikes | null;
  scanning: boolean;
  error: string | null;
  /** The pair an action is on its way for. */
  busy: string | null;
  scan: () => Promise<void>;
  decide: (pair: LookAlikePair, verdict: LookAlikeVerdict) => Promise<void>;
  /** Merges the newer model of the pair into the older as its next versions. */
  merge: (pair: LookAlikePair) => Promise<void>;
  /** Gives both models of a same name pair their own names. */
  renameBoth: (pair: LookAlikePair, first: string, second: string) => Promise<void>;
  undo: (merged: string) => Promise<void>;
  saveSettings: (settings: LookAlikeSettings) => Promise<void>;
}

export const useLookAlikesStore = create<LookAlikesStore>((set, get) => {
  const drop = (pair: LookAlikePair) =>
    set(s => ({
      report: s.report && {
        ...s.report,
        pairs: s.report.pairs.filter(p => pairKey(p) !== pairKey(pair)),
      },
    }));
  const run = async (pair: LookAlikePair, work: () => Promise<void>) => {
    set({ busy: pairKey(pair) });
    try {
      await work();
    } catch (error) {
      useAppStore.getState().notify({
        status: 'failed',
        title: 'Could not do that',
        detail: errorMessage(error),
      });
    } finally {
      set({ busy: null });
    }
  };

  return {
    report: null,
    scanning: false,
    error: null,
    busy: null,

    scan: async () => {
      set({ scanning: true, error: null });
      try {
        set({ report: await commands.libraryLookAlikes() });
      } catch (error) {
        set({ error: errorMessage(error) });
      } finally {
        set({ scanning: false });
      }
    },

    decide: (pair, verdict) =>
      run(pair, async () => {
        await commands.libraryDecide(pair.a, pair.b, verdict);
        drop(pair);
      }),

    merge: pair =>
      run(pair, async () => {
        const copyTags = get().report?.settings.copyTags ?? true;
        const done = await commands.libraryMerge({ keep: pair.a, merge: pair.b, copyTags });
        const { setProject, notify, workspace } = useAppStore.getState();
        const open = done.projects.find(p => p.id === workspace?.project?.id);
        if (open) setProject(open);
        notify({
          status: 'done',
          title: `Merged ${done.merge.mergedName} into ${done.merge.intoName}`,
          detail: `It is now ${done.merge.versions.map(v => `v${v}`).join(' and ')} of ${done.merge.intoName}.`,
          action: { label: 'Undo', run: () => void get().undo(done.merge.merged) },
        });
        await useLibraryStore.getState().load();
        await get().scan();
      }),

    renameBoth: (pair, first, second) =>
      run(pair, async () => {
        if ((await renameEntry(pair.a, first)) && (await renameEntry(pair.b, second))) drop(pair);
      }),

    undo: async merged => {
      if (await restoreEntries([merged])) await get().scan();
    },

    saveSettings: async settings => {
      try {
        const before = get().report?.settings.threshold;
        const saved = await commands.libraryLookSettings(settings);
        set(s => ({ report: s.report && { ...s.report, settings: saved } }));
        if (saved.threshold !== before) await get().scan();
      } catch (error) {
        set({ error: errorMessage(error) });
      }
    },
  };
});
