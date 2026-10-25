/**
 * Real sizes and fix-ups: each asset's saved entry with what its model
 * measures, the Inbox drafts set before Accept, and the previews that ran.
 * A saved entry changes at once (the Board and Inspector save on each
 * edit); an Inbox draft waits for Accept, so the quit guard counts it.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type AssetKind,
  type AssetPrep,
  type FixupReport,
  type ModelPrep,
  type PreviewRequest,
  type Suggestion,
} from '@/ipc/client';
import { isEmptyPrep, previewKey } from './prepare';
import { useAppStore } from './use-app-store';
import { useLayoutStore } from './use-layout-store';

export type PreviewState =
  | { status: 'running' }
  | { status: 'done'; report: FixupReport }
  | { status: 'failed'; error: string };

interface PrepareStore {
  /** The workspace the entries below belong to. */
  root: string | null;
  /** By `project/asset`. */
  assets: Readonly<Record<string, AssetPrep>>;
  /** Why an asset's entry did not load or save, by `project/asset`. */
  failed: Readonly<Record<string, string>>;
  /** What the Inbox set on a model before Accept, by inbox item id. */
  drafts: Readonly<Record<string, ModelPrep>>;
  /** By [`previewKey`]. */
  previews: Readonly<Record<string, PreviewState>>;
  /** By `kind|name`. */
  suggestions: Readonly<Record<string, Suggestion | null>>;
  saving: string | null;
  setRoot: (root: string | null) => void;
  load: (project: string, asset: string) => Promise<void>;
  /** Saves the asset's whole entry; true when it was saved. */
  save: (project: string, asset: string, entry: ModelPrep) => Promise<boolean>;
  /** Loads again every entry of `project` already shown, after an agent changed them. */
  reload: (project: string) => Promise<void>;
  /** Sets or (with an empty plan) drops an Inbox draft. */
  setDraft: (item: string, entry: ModelPrep | null) => void;
  suggest: (name: string, kind: AssetKind) => Promise<void>;
  /** Runs the plan headless on one model; a plan that already ran answers at once. */
  preview: (request: PreviewRequest) => Promise<FixupReport | null>;
}

export const assetKey = (project: string, asset: string) => `${project}/${asset}`;
export const suggestionKey = (name: string, kind: AssetKind) => `${kind}|${name}`;

/** What a preview runs on, for its key. */
export function previewTarget(r: Pick<PreviewRequest, 'project' | 'asset' | 'inbox'>): string {
  return r.inbox ? `inbox:${r.inbox}:${r.project ?? ''}` : `asset:${r.project}/${r.asset}`;
}

function without<T>(map: Readonly<Record<string, T>>, key: string): Record<string, T> {
  const { [key]: _gone, ...rest } = map;
  return rest;
}

export const usePrepareStore = create<PrepareStore>((set, get) => {
  const put = (found: AssetPrep) =>
    set(s => ({
      assets: { ...s.assets, [assetKey(found.project, found.asset)]: found },
      failed: without(s.failed, assetKey(found.project, found.asset)),
    }));

  return {
    root: null,
    assets: {},
    failed: {},
    drafts: {},
    previews: {},
    suggestions: {},
    saving: null,

    setRoot: root => {
      if (get().root !== root) {
        set({ root, assets: {}, failed: {}, drafts: {}, previews: {}, suggestions: {} });
      }
    },

    load: async (project, asset) => {
      const root = get().root;
      try {
        const found = await commands.prepareAsset(project, asset);
        if (get().root === root) put(found);
      } catch (error) {
        if (get().root !== root) return;
        set(s => ({ failed: { ...s.failed, [assetKey(project, asset)]: errorMessage(error) } }));
      }
    },

    save: async (project, asset, entry) => {
      const key = assetKey(project, asset);
      if (get().saving === key) return false;
      // Shown at once; the answer (or a reload after a refusal) settles it.
      const before = get().assets[key];
      if (before) put({ ...before, entry });
      set({ saving: key });
      try {
        put(await commands.prepareSave(project, asset, entry));
        void useLayoutStore.getState().refreshModels(project);
        return true;
      } catch (error) {
        if (before) put(before);
        useAppStore.getState().notify({
          status: 'failed',
          title: 'Real size and fix-ups not saved',
          detail: errorMessage(error),
        });
        return false;
      } finally {
        set({ saving: null });
      }
    },

    reload: async project => {
      const shown = Object.values(get().assets).filter(a => a.project === project);
      await Promise.all([
        ...shown.map(a => get().load(project, a.asset)),
        useLayoutStore.getState().refreshModels(project),
      ]);
    },

    setDraft: (item, entry) =>
      set(s => ({
        drafts: isEmptyPrep(entry)
          ? without(s.drafts, item)
          : { ...s.drafts, [item]: entry as ModelPrep },
      })),

    suggest: async (name, kind) => {
      const key = suggestionKey(name, kind);
      if (key in get().suggestions) return;
      try {
        const found = await commands.prepareSuggest(name, kind);
        set(s => ({ suggestions: { ...s.suggestions, [key]: found } }));
      } catch {
        // No suggestion is fine; the size field still works.
      }
    },

    preview: async request => {
      const key = previewKey(previewTarget(request), request.plan);
      const known = get().previews[key];
      if (known?.status === 'done') return known.report;
      if (known?.status === 'running') return null;
      const root = get().root;
      set(s => ({ previews: { ...s.previews, [key]: { status: 'running' } } }));
      try {
        const report = await commands.preparePreview(request);
        if (get().root !== root) return null;
        set(s => ({ previews: { ...s.previews, [key]: { status: 'done', report } } }));
        return report;
      } catch (error) {
        if (get().root !== root) return null;
        const failed: PreviewState = { status: 'failed', error: errorMessage(error) };
        set(s => ({ previews: { ...s.previews, [key]: failed } }));
        return null;
      }
    },
  };
});
