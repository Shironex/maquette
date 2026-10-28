/**
 * The workspace's animation library: its clips, the view's filters, the
 * Import clips sheet and the Save clips to the library sheet. Clips live
 * once outside any project, skeleton only; reading files runs Blender
 * headless and nothing is added until the person imports or saves.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type AnimationsView,
  type ClipsFrom,
  type ImportPlan,
  type SavePlan,
} from '@/ipc/client';
import {
  NO_ANIMATION_FILTERS,
  type AnimationFilters,
  type AnimationLayout,
  type ImportChoices,
} from './animations';
import { useAppStore } from './use-app-store';

export interface ImportSheet {
  /** Every file added so far; adding more reads them all again. */
  paths: string[];
  plan: ImportPlan | null;
  reading: boolean;
  error: string | null;
  choices: ImportChoices;
  /** Pack names by zip, where the person changed them. */
  packs: Record<string, string>;
  singles: string;
  keepFiles: boolean;
}

export interface SaveSheet {
  from: ClipsFrom;
  plan: SavePlan | null;
  error: string | null;
  /** Clip names to save. */
  chosen: string[];
  pack: string;
  tags: string[];
}

const NO_CHOICES: ImportChoices = { keepBoth: [], skip: [] };

interface AnimationsStore {
  view: AnimationsView | null;
  error: string | null;
  filters: AnimationFilters;
  layout: AnimationLayout;
  /** The clip the side panel shows. */
  selected: string | null;
  importing: ImportSheet | null;
  saving: SaveSheet | null;
  /** An import or save is on its way. */
  busy: boolean;

  load: () => Promise<void>;
  /** Reloads when the library was loaded. */
  onChanged: () => void;
  setFilters: (change: Partial<AnimationFilters>) => void;
  setLayout: (layout: AnimationLayout) => void;
  select: (clip: string | null) => void;

  openImport: () => void;
  closeImport: () => void;
  /** Reads `paths` along with the files already added. */
  addFiles: (paths: readonly string[]) => Promise<void>;
  /** Picks FBX files or zips with the system dialog and adds them. */
  pickFiles: () => Promise<void>;
  changeImport: (change: Partial<Omit<ImportSheet, 'paths' | 'plan' | 'reading'>>) => void;
  /** Adds the clips as chosen. */
  runImport: () => Promise<void>;

  openSave: (from: ClipsFrom) => Promise<void>;
  closeSave: () => void;
  changeSave: (change: Partial<Pick<SaveSheet, 'chosen' | 'pack' | 'tags'>>) => void;
  runSave: () => Promise<void>;
}

const emptyImport = (): ImportSheet => ({
  paths: [],
  plan: null,
  reading: false,
  error: null,
  choices: NO_CHOICES,
  packs: {},
  singles: 'Singles',
  keepFiles: false,
});

export const useAnimationsStore = create<AnimationsStore>((set, get) => ({
  view: null,
  error: null,
  filters: NO_ANIMATION_FILTERS,
  layout: 'grid',
  selected: null,
  importing: null,
  saving: null,
  busy: false,

  load: async () => {
    try {
      const view = await commands.animationsGet();
      const selected = get().selected;
      const kept = view.items.some(i => i.clip.id === selected) ? selected : null;
      set({ view, error: null, selected: kept });
    } catch (error) {
      set({ error: errorMessage(error) });
    }
  },

  onChanged: () => {
    if (get().view) void get().load();
  },

  setFilters: change => set(s => ({ filters: { ...s.filters, ...change } })),
  setLayout: layout => set({ layout }),
  select: selected => set({ selected }),

  openImport: () => set({ importing: emptyImport() }),
  closeImport: () => set({ importing: null }),

  addFiles: async paths => {
    const sheet = get().importing ?? emptyImport();
    const all = [...new Set([...sheet.paths, ...paths])];
    set({ importing: { ...sheet, paths: all, reading: true, error: null } });
    try {
      const plan = await commands.animationsReadImport(all);
      const now = get().importing;
      if (now?.paths === all) set({ importing: { ...now, plan, reading: false } });
    } catch (error) {
      const now = get().importing;
      if (now?.paths === all) {
        set({
          importing: { ...now, paths: sheet.paths, reading: false, error: errorMessage(error) },
        });
      }
    }
  },

  pickFiles: async () => {
    try {
      const paths = await commands.dialogPickClips('Add FBX files or pack zips');
      if (paths.length) await get().addFiles(paths);
    } catch (error) {
      useAppStore.getState().notify({
        status: 'failed',
        title: 'Could not open the files',
        detail: errorMessage(error),
      });
    }
  },

  changeImport: change => {
    const sheet = get().importing;
    if (sheet) set({ importing: { ...sheet, ...change } });
  },

  runImport: async () => {
    const sheet = get().importing;
    if (!sheet?.plan) return;
    const { notify } = useAppStore.getState();
    set({ busy: true });
    try {
      const done = await commands.animationsImport({
        token: sheet.plan.token,
        packs: Object.entries(sheet.packs).map(([file, pack]) => ({ file, pack })),
        singles: sheet.singles,
        keepFiles: sheet.keepFiles,
        keepBoth: [...sheet.choices.keepBoth],
        skip: [...sheet.choices.skip],
      });
      const n = done.added.length;
      const revised = done.revised.length;
      notify({
        status: 'done',
        title: `Imported ${n} ${n === 1 ? 'clip' : 'clips'} into the library`,
        detail: revised
          ? `${revised} replaced an older take; models that baked it are offered an update.`
          : undefined,
      });
      set({ importing: null, selected: done.added[0]?.id ?? get().selected });
      await get().load();
    } catch (error) {
      set({ importing: { ...sheet, error: errorMessage(error) } });
    } finally {
      set({ busy: false });
    }
  },

  openSave: async from => {
    const sheet: SaveSheet = { from, plan: null, error: null, chosen: [], pack: '', tags: [] };
    set({ saving: sheet });
    try {
      const plan = await commands.animationsSavePlan(from);
      if (get().saving !== sheet) return;
      set({
        saving: {
          ...sheet,
          plan,
          chosen: plan.clips.filter(c => !c.fromLibrary).map(c => c.name),
          pack: plan.pack,
          tags: plan.tags,
        },
      });
    } catch (error) {
      if (get().saving === sheet) set({ saving: { ...sheet, error: errorMessage(error) } });
    }
  },

  closeSave: () => set({ saving: null }),

  changeSave: change => {
    const sheet = get().saving;
    if (sheet) set({ saving: { ...sheet, ...change } });
  },

  runSave: async () => {
    const sheet = get().saving;
    if (!sheet?.plan) return;
    const { notify } = useAppStore.getState();
    set({ busy: true });
    try {
      const saved = await commands.animationsSave({
        from: sheet.from,
        clips: sheet.chosen,
        pack: sheet.pack,
        tags: sheet.tags,
      });
      const n = saved.added.length;
      notify({
        status: 'done',
        title: `Saved ${n} ${n === 1 ? 'clip' : 'clips'} to ${sheet.pack}`,
        detail: saved.skipped.length
          ? `The library had ${saved.skipped.join(', ')} already.`
          : undefined,
      });
      set({ saving: null });
      await get().load();
    } catch (error) {
      set({ saving: { ...sheet, error: errorMessage(error) } });
    } finally {
      set({ busy: false });
    }
  },
}));
