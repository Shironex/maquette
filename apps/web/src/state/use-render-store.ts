/**
 * Renders in the app: the batch sheet (its source, rows and settings), the
 * render it follows, kept up to date by the render changed event, and the
 * renders before it. The core plans, renders and writes every file.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type PackSource,
  type RenderPlan,
  type RenderRecord,
  type RenderRow,
  type RenderSettings,
  type RenderStyle,
} from '@/ipc/client';
import { RENDER_DEFAULTS } from './renders';
import { useAppStore } from './use-app-store';

/** The styles a row starts with: clay always works, toon needs colour. */
const FIRST_STYLES: readonly RenderStyle[] = ['clay'];

interface RenderStore {
  open: boolean;
  source: PackSource | null;
  /** "Opened from the Board selection". */
  from: string;
  plan: RenderPlan | null;
  /** The keys the selection started with; empty for a whole source. */
  selection: string[];
  scope: 'selected' | 'all';
  /** The ticked assets and what each renders. */
  rows: Record<string, RenderStyle[]>;
  ticked: string[];
  settings: Required<RenderSettings>;
  replace: boolean;
  sheet: boolean;
  error: string | null;
  busy: boolean;
  /** The render the sheet follows, and the newest of any for the status bar. */
  record: RenderRecord | null;
  live: RenderRecord | null;
  recent: RenderRecord[];
  /** Settings was asked to jump to the Toolchain section. */
  toolchainAsked: boolean;

  show: (source: PackSource, selection: string[], from: string) => Promise<void>;
  /** Adds one style of one asset, opening the sheet on the project. */
  addToBatch: (
    source: PackSource,
    key: string,
    style: RenderStyle,
    settings: Required<RenderSettings>
  ) => Promise<void>;
  close: () => void;
  setScope: (scope: 'selected' | 'all') => void;
  tick: (key: string, on: boolean) => void;
  toggleStyle: (key: string, style: RenderStyle) => void;
  applyToAll: (style: RenderStyle) => void;
  change: (change: Partial<RenderSettings>) => void;
  setReplace: (replace: boolean) => void;
  setSheet: (sheet: boolean) => void;
  start: () => Promise<void>;
  /** Renders one asset's style, outside the sheet. */
  renderOne: (
    source: PackSource,
    key: string,
    style: RenderStyle,
    settings: RenderSettings
  ) => Promise<void>;
  stop: () => Promise<void>;
  resume: () => Promise<void>;
  reveal: () => Promise<void>;
  loadRecent: () => Promise<void>;
  onChanged: (record: RenderRecord) => void;
  openToolchain: () => void;
  toolchainShown: () => void;
}

const notifyFailed = (title: string, error: unknown) =>
  useAppStore.getState().notify({ status: 'failed', title, detail: errorMessage(error) });

function rowsOf(plan: RenderPlan, keys: readonly string[]): Record<string, RenderStyle[]> {
  const out: Record<string, RenderStyle[]> = {};
  for (const target of plan.targets) {
    if (keys.includes(target.key)) out[target.key] = [...FIRST_STYLES];
  }
  return out;
}

const upsert = (list: readonly RenderRecord[], record: RenderRecord): RenderRecord[] =>
  [record, ...list.filter(r => r.id !== record.id)].sort((a, b) =>
    b.createdAt.localeCompare(a.createdAt)
  );

export const useRenderStore = create<RenderStore>((set, get) => ({
  open: false,
  source: null,
  from: '',
  plan: null,
  selection: [],
  scope: 'selected',
  rows: {},
  ticked: [],
  settings: RENDER_DEFAULTS,
  replace: true,
  sheet: true,
  error: null,
  busy: false,
  record: null,
  live: null,
  recent: [],
  toolchainAsked: false,

  show: async (source, selection, from) => {
    set({
      open: true,
      source,
      from,
      selection,
      scope: selection.length > 0 ? 'selected' : 'all',
      plan: null,
      rows: {},
      ticked: [],
      error: null,
      record:
        get().recent.find(
          r => r.state === 'running' && JSON.stringify(r.source) === JSON.stringify(source)
        ) ?? null,
    });
    try {
      const plan = await commands.renderPlan(source);
      const keys = selection.length > 0 ? selection : plan.targets.map(t => t.key);
      const usable = plan.targets.filter(t => keys.includes(t.key) && !t.unreadable);
      set({
        plan,
        rows: rowsOf(plan, keys),
        ticked: usable.map(t => t.key),
      });
    } catch (error) {
      set({ error: errorMessage(error) });
    }
  },

  addToBatch: async (source, key, style, settings) => {
    const state = get();
    const same = state.open && JSON.stringify(state.source) === JSON.stringify(source);
    if (!same) await get().show(source, [key], 'Added from the Render tab');
    set(s => ({
      open: true,
      settings,
      ticked: s.ticked.includes(key) ? s.ticked : [...s.ticked, key],
      rows: { ...s.rows, [key]: [...new Set([...(s.rows[key] ?? []), style])] },
    }));
  },

  close: () => set({ open: false }),

  setScope: scope => {
    const { plan, selection } = get();
    if (!plan) return;
    const keys = scope === 'selected' ? selection : plan.targets.map(t => t.key);
    const usable = plan.targets.filter(t => keys.includes(t.key) && !t.unreadable);
    set(s => ({
      scope,
      ticked: usable.map(t => t.key),
      rows: { ...rowsOf(plan, keys), ...s.rows },
    }));
  },

  tick: (key, on) =>
    set(s => ({
      ticked: on ? [...new Set([...s.ticked, key])] : s.ticked.filter(k => k !== key),
      rows: s.rows[key] ? s.rows : { ...s.rows, [key]: [...FIRST_STYLES] },
    })),

  toggleStyle: (key, style) =>
    set(s => {
      const now = s.rows[key] ?? [];
      const next = now.includes(style) ? now.filter(x => x !== style) : [...now, style];
      return { rows: { ...s.rows, [key]: next } };
    }),

  applyToAll: style =>
    set(s => {
      const on = s.ticked.every(k => s.rows[k]?.includes(style));
      const rows = { ...s.rows };
      for (const key of s.ticked) {
        const now = rows[key] ?? [];
        rows[key] = on ? now.filter(x => x !== style) : [...new Set([...now, style])];
      }
      return { rows };
    }),

  change: change => set(s => ({ settings: { ...s.settings, ...change } })),
  setReplace: replace => set({ replace }),
  setSheet: sheet => set({ sheet }),

  start: async () => {
    const { source, rows, ticked, settings, replace, sheet } = get();
    if (!source) return;
    const picked: RenderRow[] = ticked
      .filter(key => (rows[key]?.length ?? 0) > 0)
      .map(key => ({ key, styles: rows[key] ?? [] }));
    set({ busy: true, error: null });
    try {
      const record = await commands.renderStart({
        source,
        rows: picked,
        settings,
        replace,
        sheet: sheet && picked.length > 1,
      });
      set(s => ({ record, live: record, recent: upsert(s.recent, record) }));
    } catch (error) {
      set({ error: errorMessage(error) });
    } finally {
      set({ busy: false });
    }
  },

  renderOne: async (source, key, style, settings) => {
    set({ busy: true });
    try {
      const record = await commands.renderStart({
        source,
        rows: [{ key, styles: [style] }],
        settings,
        replace: true,
        sheet: false,
      });
      set(s => ({ live: record, recent: upsert(s.recent, record) }));
    } catch (error) {
      notifyFailed('Could not start the render', error);
    } finally {
      set({ busy: false });
    }
  },

  stop: async () => {
    const id = get().record?.id ?? get().live?.id;
    if (!id) return;
    try {
      get().onChanged(await commands.renderStop(id));
    } catch (error) {
      notifyFailed('Could not stop the render', error);
    }
  },

  resume: async () => {
    const id = get().record?.id;
    if (!id) return;
    try {
      get().onChanged(await commands.renderResume(id));
    } catch (error) {
      notifyFailed('Could not go on with the render', error);
    }
  },

  reveal: async () => {
    const id = get().record?.id ?? get().recent[0]?.id;
    if (!id) return;
    try {
      await commands.renderReveal(id);
    } catch (error) {
      notifyFailed('Could not open the exports folder', error);
    }
  },

  loadRecent: async () => {
    try {
      const recent = await commands.renderRecent();
      const running = recent.find(r => r.state === 'running') ?? null;
      set(s => ({ recent, live: running ?? s.live }));
    } catch {
      // No project open yet: nothing rendered before it to show.
    }
  },

  onChanged: record =>
    set(s => ({
      record: s.record?.id === record.id ? record : s.record,
      live: record.state === 'running' || s.live?.id === record.id ? record : s.live,
      recent: upsert(s.recent, record),
    })),

  openToolchain: () => {
    set({ toolchainAsked: true });
    useAppStore.getState().setView('settings');
  },
  toolchainShown: () => set({ toolchainAsked: false }),
}));
