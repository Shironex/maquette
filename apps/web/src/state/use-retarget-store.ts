/**
 * Putting library clips on a rigged model, in four steps: choose clips,
 * check the bone map, set each clip up for this model, and review a
 * headless bake before it becomes the model's next version. Opened from
 * the Animations library (Use on model) or from Motion (Add from library,
 * Map bones). Nothing is saved until the person bakes.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type ClipSetting,
  type MapChoice,
  type RetargetDraft,
  type RetargetPlan,
} from '@/ipc/client';
import { useAnimationsStore } from './use-animations-store';
import { useAppStore } from './use-app-store';
import { useMotionStore } from './use-motion-store';
import { withSetting, type RetargetStep } from './retarget';

export interface RetargetSheet {
  project: string;
  asset: string | null;
  step: RetargetStep;
  /** Library clip ids, in the order chosen. */
  chosen: string[];
  plan: RetargetPlan | null;
  planning: boolean;
  /** Bones mapped by hand in this retarget. */
  picked: MapChoice[];
  saveMap: boolean;
  settings: ClipSetting[];
  /** The clip the settings and review panels show. */
  selected: string | null;
  draft: RetargetDraft | null;
  error: string | null;
  query: string;
  pack: string | null;
}

export interface OpenRetarget {
  project: string;
  asset?: string | null;
  clips?: string[];
  step?: RetargetStep;
}

type Editable = Pick<RetargetSheet, 'query' | 'pack' | 'saveMap' | 'selected'>;

interface RetargetStore {
  sheet: RetargetSheet | null;
  /** A bake or the version it makes is on its way. */
  busy: boolean;
  open: (options: OpenRetarget) => Promise<void>;
  close: () => void;
  change: (change: Partial<Editable>) => void;
  toggle: (clip: string) => void;
  setAsset: (asset: string) => Promise<void>;
  /** Reads the plan again for the chosen clips and the map so far. */
  refresh: () => Promise<void>;
  goTo: (step: RetargetStep) => void;
  next: () => Promise<void>;
  back: () => void;
  pick: (bone: string, target: string | null) => Promise<void>;
  changeSetting: (clip: string, change: Partial<ClipSetting>) => void;
  resetNames: () => void;
  /** Bakes the clips onto a copy, headless, and opens the review. */
  retarget: () => Promise<void>;
  /** Turns Pin the feet on for one clip and bakes again. */
  pinFeet: (clip: string) => Promise<void>;
  /** Opens the reviewed bake in Blender's window, on a click. */
  openInBlender: () => Promise<void>;
  /** Adds the reviewed bake as the model's next version. */
  bake: () => Promise<void>;
}

const ORDER: RetargetStep[] = ['choose', 'map', 'settings', 'review'];

export const useRetargetStore = create<RetargetStore>((set, get) => {
  const patch = (change: Partial<RetargetSheet>) => {
    const sheet = get().sheet;
    if (sheet) set({ sheet: { ...sheet, ...change } });
  };

  return {
    sheet: null,
    busy: false,

    open: async ({ project, asset = null, clips = [], step = 'choose' }) => {
      if (useMotionStore.getState().view?.project !== project) {
        void useMotionStore.getState().load(project);
      }
      set({
        sheet: {
          project,
          asset,
          step,
          chosen: clips,
          plan: null,
          planning: false,
          picked: [],
          saveMap: true,
          settings: [],
          selected: clips[0] ?? null,
          draft: null,
          error: null,
          query: '',
          pack: null,
        },
      });
      if (asset) await get().refresh();
    },

    close: () => set({ sheet: null }),
    change: change => patch(change),

    toggle: clip => {
      const sheet = get().sheet;
      if (!sheet) return;
      const chosen = sheet.chosen.includes(clip)
        ? sheet.chosen.filter(c => c !== clip)
        : [...sheet.chosen, clip];
      patch({ chosen, draft: null });
    },

    setAsset: async asset => {
      patch({ asset, picked: [], settings: [], plan: null, draft: null });
      await get().refresh();
    },

    refresh: async () => {
      const sheet = get().sheet;
      if (!sheet?.asset) return;
      patch({ planning: true, error: null });
      try {
        const plan = await commands.animationsRetargetPlan(
          sheet.project,
          sheet.asset,
          sheet.chosen,
          sheet.picked
        );
        const now = get().sheet;
        if (!now || now.asset !== sheet.asset) return;
        const kept = (s: ClipSetting) => now.settings.find(o => o.clip === s.clip) ?? s;
        patch({
          plan,
          planning: false,
          settings: plan.settings.map(kept),
          selected: now.selected ?? plan.settings[0]?.clip ?? null,
        });
      } catch (error) {
        patch({ planning: false, error: errorMessage(error) });
      }
    },

    goTo: step => patch({ step }),

    next: async () => {
      const sheet = get().sheet;
      if (!sheet) return;
      if (sheet.step === 'choose') {
        await get().refresh();
        const plan = get().sheet?.plan;
        if (plan) patch({ step: plan.needsLook ? 'map' : 'settings' });
        return;
      }
      if (sheet.step === 'settings') return get().retarget();
      const at = ORDER.indexOf(sheet.step);
      patch({ step: ORDER[Math.min(at + 1, ORDER.length - 1)] });
    },

    back: () => {
      const sheet = get().sheet;
      if (sheet) patch({ step: ORDER[Math.max(ORDER.indexOf(sheet.step) - 1, 0)] });
    },

    pick: async (bone, target) => {
      const sheet = get().sheet;
      if (!sheet) return;
      const picked = [...sheet.picked.filter(p => p.bone !== bone), { bone, target }];
      patch({ picked });
      await get().refresh();
    },

    changeSetting: (clip, change) => {
      const sheet = get().sheet;
      if (sheet) patch({ settings: withSetting(sheet.settings, clip, change), draft: null });
    },

    resetNames: () => {
      const sheet = get().sheet;
      const plan = sheet?.plan;
      if (!sheet || !plan) return;
      patch({
        settings: sheet.settings.map(s => ({
          ...s,
          name: plan.settings.find(p => p.clip === s.clip)?.name ?? s.name,
        })),
      });
    },

    retarget: async () => {
      const sheet = get().sheet;
      if (!sheet?.asset || !sheet.settings.length) return;
      set({ busy: true });
      patch({ error: null });
      try {
        const draft = await commands.animationsRetarget({
          project: sheet.project,
          asset: sheet.asset,
          settings: sheet.settings,
          map: sheet.picked,
          saveMap: sheet.saveMap,
        });
        patch({
          draft,
          step: 'review',
          settings: draft.settings,
          selected: draft.review.clips[0]?.clip ?? null,
        });
      } catch (error) {
        patch({ error: errorMessage(error) });
      } finally {
        set({ busy: false });
      }
    },

    pinFeet: async clip => {
      const sheet = get().sheet;
      if (!sheet) return;
      patch({ settings: withSetting(sheet.settings, clip, { pinFeet: true }), selected: clip });
      await get().retarget();
      patch({ selected: clip });
    },

    openInBlender: async () => {
      const token = get().sheet?.draft?.token;
      if (!token) return;
      try {
        await commands.animationsOpenDraftInBlender(token);
      } catch (error) {
        useAppStore.getState().notify({
          status: 'failed',
          title: 'Could not open Blender',
          detail: errorMessage(error),
        });
      }
    },

    bake: async () => {
      const sheet = get().sheet;
      if (!sheet?.draft) return;
      const { notify, setProject } = useAppStore.getState();
      set({ busy: true });
      try {
        const baked = await commands.animationsBake(sheet.draft.token);
        setProject(baked.project);
        const n = baked.review.clips.filter(c => c.status !== 'failed').length;
        notify({
          status: 'done',
          title: `Baked ${n} ${n === 1 ? 'clip' : 'clips'} into ${sheet.draft.name} v${baked.version.version}`,
          detail: baked.warnings.length
            ? `Checks to look at: ${baked.warnings.join(', ')}.`
            : `v${sheet.draft.fromVersion} is kept.`,
        });
        set({ sheet: null });
        useAnimationsStore.getState().onChanged();
      } catch (error) {
        patch({ error: errorMessage(error) });
      } finally {
        set({ busy: false });
      }
    },
  };
});
