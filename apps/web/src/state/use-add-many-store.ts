/**
 * Add a whole project to the library: every accepted model in one go, with
 * the names and tags settled before anything is copied. Same file models
 * are skipped without asking, a model that meets one already in the library
 * is asked about once up front, unreadable files stay in the project, and
 * Stop finishes the model being copied first. It spends no credits.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type AddManyPlan,
  type AssetKind,
  type PlannedAdd,
} from '@/ipc/client';
import { useAppStore } from './use-app-store';
import { useLibraryStore } from './use-library-store';
import {
  type ClashChoice,
  type AddManyOptions,
  NEW_COLLECTION,
  defaultChoice,
  plannedTags,
  renamedName,
} from './add-many';

export interface AddManyProgress {
  done: number;
  total: number;
  current: string | null;
  stopping: boolean;
}

export interface AddManyResult {
  added: number;
  skipped: number;
  failed: { name: string; why: string }[];
  stopped: boolean;
}

interface AddManyStore {
  open: boolean;
  project: string | null;
  plan: AddManyPlan | null;
  error: string | null;
  options: AddManyOptions;
  choices: Record<string, ClashChoice>;
  progress: AddManyProgress | null;
  result: AddManyResult | null;
  show: (project: string | null) => void;
  close: () => void;
  pick: (project: string) => Promise<void>;
  setOptions: (change: Partial<AddManyOptions>) => void;
  setKindTags: (kind: AssetKind, text: string) => void;
  choose: (asset: string, choice: ClashChoice) => void;
  start: () => Promise<void>;
  stop: () => void;
}

const FRESH: AddManyOptions = {
  projectTag: true,
  tag: '',
  collection: '',
  newCollection: '',
  kindTags: { prop: 'prop', character: 'character', scene: 'scene' },
};

/** One add, settled: what to ask the core for, or null to leave it out. */
function request(p: PlannedAdd, choice: ClashChoice | undefined, names: readonly string[]) {
  if (p.state === 'same-file' || p.state === 'unreadable') return null;
  if (p.state === 'name-clash' || p.state === 'looks-like') {
    if (choice === 'skip') return null;
    if (choice === 'version') return { into: p.entry, name: null };
    return { into: null, name: p.state === 'name-clash' ? renamedName(p.name, names) : null };
  }
  return { into: null, name: null };
}

export const useAddManyStore = create<AddManyStore>((set, get) => ({
  open: false,
  project: null,
  plan: null,
  error: null,
  options: FRESH,
  choices: {},
  progress: null,
  result: null,

  show: project => {
    set({ open: true, plan: null, error: null, result: null, progress: null, options: FRESH });
    if (project) void get().pick(project);
  },
  close: () => {
    if (get().progress) return;
    set({ open: false, project: null, plan: null, result: null });
  },

  pick: async project => {
    set({ project, plan: null, error: null, result: null, choices: {} });
    try {
      const plan = await commands.libraryAddManyPlan(project);
      if (get().project !== project) return;
      const choices = Object.fromEntries(
        plan.assets.flatMap(a => {
          const choice = defaultChoice(a);
          return choice ? [[a.asset, choice]] : [];
        })
      );
      set(s => ({ plan, choices, options: { ...s.options, tag: plan.tag } }));
    } catch (error) {
      set({ error: errorMessage(error) });
    }
  },

  setOptions: change => set(s => ({ options: { ...s.options, ...change } })),
  setKindTags: (kind, text) =>
    set(s => ({ options: { ...s.options, kindTags: { ...s.options.kindTags, [kind]: text } } })),
  choose: (asset, choice) => set(s => ({ choices: { ...s.choices, [asset]: choice } })),

  start: async () => {
    const { plan, options, choices } = get();
    if (!plan || get().progress) return;
    const names = useLibraryStore.getState().view?.items.map(i => i.entry.name) ?? [];
    const work = plan.assets.flatMap(p => {
      const r = request(p, choices[p.asset], names);
      return r ? [{ p, ...r }] : [];
    });
    const result: AddManyResult = {
      added: 0,
      skipped: plan.assets.length - work.length,
      failed: [],
      stopped: false,
    };
    const tagged = new Map<string, string[]>();
    set({ progress: { done: 0, total: work.length, current: null, stopping: false } });
    for (const [i, item] of work.entries()) {
      if (get().progress?.stopping) {
        result.stopped = true;
        break;
      }
      set(s => ({ progress: s.progress && { ...s.progress, done: i, current: item.p.name } }));
      try {
        const added = await commands.libraryAdd({
          project: plan.project,
          asset: item.p.asset,
          name: item.name,
          tags: null,
          into: item.into,
        });
        result.added += 1;
        const key = plannedTags(item.p.kind, options).join(',');
        tagged.set(key, [...(tagged.get(key) ?? []), added.entry.id]);
        if (i === work.length - 1) useAppStore.getState().setProject(added.project);
      } catch (error) {
        result.failed.push({ name: item.p.name, why: errorMessage(error) });
      }
    }
    await settle(tagged, options);
    await useLibraryStore.getState().load();
    set({ progress: null, result });
  },

  stop: () => set(s => ({ progress: s.progress && { ...s.progress, stopping: true } })),
}));

/** Tags every added model and puts it in the collection, once the copies are done. */
async function settle(tagged: Map<string, string[]>, options: AddManyOptions) {
  const entries = [...tagged.values()].flat();
  if (entries.length === 0) return;
  try {
    for (const [key, ids] of tagged) {
      if (key) await commands.libraryTag({ entries: ids, add: key.split(','), remove: [] });
    }
    if (options.collection === NEW_COLLECTION && options.newCollection.trim()) {
      await commands.libraryCollection({ op: 'create', name: options.newCollection, entries });
    } else if (options.collection && options.collection !== NEW_COLLECTION) {
      await commands.libraryCollection({ op: 'add', collection: options.collection, entries });
    }
  } catch (error) {
    useAppStore.getState().notify({
      status: 'failed',
      title: 'The models are in, but the tags or collection were not set',
      detail: errorMessage(error),
    });
  }
}
