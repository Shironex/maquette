/**
 * The Motion view's state: the project's motion as the app read it, the
 * draft of object states and clip choices, what is selected and the
 * playhead the viewport and the timeline share.
 */
import { create } from 'zustand';
import { commands, errorMessage, type MotionView, type ObjectStates } from '@/ipc/client';
import { useAppStore } from './use-app-store';
import { usePipelineStore } from './use-pipeline-store';
import {
  EMPTY_DRAFT,
  advance,
  draftOf,
  n,
  sameDraft,
  withShip,
  type MotionDraft,
  type MotionFilter,
  type MotionTab,
  type Speed,
} from './motion';
import { firstStates } from './motion-states';

export interface Player {
  playing: boolean;
  /** Seconds into the clip. */
  time: number;
  speed: Speed;
  loop: boolean;
  /** The preview loop, in frames: In and Out. The clip ships whole. */
  range: [number, number];
}

const PLAYER: Player = { playing: true, time: 0, speed: '1', loop: true, range: [0, 0] };

interface MotionStore {
  view: MotionView | null;
  project: string | null;
  loading: boolean;
  saving: boolean;
  error: string | null;
  saved: MotionDraft;
  draft: MotionDraft;
  /** An agent changed the layout while the draft had changes of its own. */
  outside: boolean;
  filter: MotionFilter;
  /** The selected model, its tab, clip and state. */
  asset: string | null;
  tab: MotionTab;
  clip: string | null;
  state: string | null;
  player: Player;
  load: (project: string) => Promise<void>;
  /** The layout or the project changed outside the view. */
  onChanged: (project: string) => Promise<void>;
  setFilter: (filter: MotionFilter) => void;
  select: (asset: string) => void;
  setTab: (tab: MotionTab) => void;
  selectClip: (clip: string) => void;
  selectState: (state: string) => void;
  setShip: (asset: string, clip: string, ship: boolean) => void;
  /** Changes one asset's states; an empty set drops them. */
  editStates: (asset: string, change: (object: ObjectStates) => ObjectStates) => void;
  /** Gives a model its first state, the rest pose, and opens States. */
  giveStates: (asset: string) => void;
  setPlayer: (patch: Partial<Player>) => void;
  /** Moves the playhead on by `delta` seconds of wall time. */
  tick: (delta: number) => void;
  revert: () => void;
  save: () => Promise<boolean>;
}

let requested: string | null = null;

/** The clip the player plays, or none. */
export function currentClip(s: Pick<MotionStore, 'view' | 'asset' | 'clip'>) {
  const model = s.view?.models.find(m => m.asset === s.asset);
  return model?.clips.find(c => c.name === s.clip) ?? null;
}

export const useMotionStore = create<MotionStore>((set, get) => {
  const take = (view: MotionView) => {
    const draft = draftOf(view);
    set({ view, saved: draft, draft, outside: false, loading: false });
    const { asset } = get();
    if (view.models.some(m => m.asset === asset)) return;
    const first =
      view.models.find(m => m.rig || m.clips.length > 0) ??
      view.models.find(m => view.states[m.asset]);
    if (first) get().select(first.asset);
    else set({ asset: null, clip: null, state: null });
  };

  const resetPlayer = (frames: number) =>
    set({ player: { ...get().player, time: 0, range: [0, frames] } });

  return {
    view: null,
    project: null,
    loading: false,
    saving: false,
    error: null,
    saved: EMPTY_DRAFT,
    draft: EMPTY_DRAFT,
    outside: false,
    filter: 'all',
    asset: null,
    tab: 'clips',
    clip: null,
    state: null,
    player: PLAYER,

    load: async project => {
      requested = project;
      if (get().project !== project) {
        set({ view: null, project, asset: null, saved: EMPTY_DRAFT, draft: EMPTY_DRAFT });
      }
      set({ loading: true, error: null });
      try {
        const view = await commands.motionGet(project);
        if (requested === project) take(view);
      } catch (error) {
        if (requested === project) set({ loading: false, error: errorMessage(error) });
      }
    },

    onChanged: async project => {
      if (get().project !== project) return;
      if (!sameDraft(get().draft, get().saved)) {
        set({ outside: true });
        return;
      }
      await get().load(project);
    },

    setFilter: filter => set({ filter }),

    select: asset => {
      const model = get().view?.models.find(m => m.asset === asset);
      const object = get().draft.states[asset];
      const clip = model?.clips[0] ?? null;
      const tab: MotionTab = !model?.rig && !clip && object ? 'states' : 'clips';
      set({ asset, tab, clip: clip?.name ?? null, state: object?.states[1]?.id ?? null });
      resetPlayer(clip?.frames ?? 0);
    },

    setTab: tab => set({ tab }),

    selectClip: name => {
      const model = get().view?.models.find(m => m.asset === get().asset);
      const clip = model?.clips.find(c => c.name === name);
      set({ clip: clip?.name ?? null });
      resetPlayer(clip?.frames ?? 0);
    },

    selectState: state => set({ state }),

    setShip: (asset, clip, ship) => set({ draft: withShip(get().draft, asset, clip, ship) }),

    editStates: (asset, change) => {
      const draft = get().draft;
      const current = draft.states[asset];
      if (!current) return;
      const next = change(current);
      const states = { ...draft.states };
      if (next.states.length > 0) states[asset] = next;
      else delete states[asset];
      set({ draft: { ...draft, states } });
      if (!next.states.some(s => s.id === get().state)) set({ state: next.states[0]?.id ?? null });
    },

    giveStates: asset => {
      const draft = get().draft;
      if (!draft.states[asset]) {
        set({ draft: { ...draft, states: { ...draft.states, [asset]: firstStates() } } });
      }
      get().select(asset);
      set({ tab: 'states', state: get().draft.states[asset]?.states[0]?.id ?? null });
    },

    setPlayer: patch => set({ player: { ...get().player, ...patch } }),

    tick: delta => {
      const { player } = get();
      const clip = currentClip(get());
      if (!player.playing || !clip) return;
      const fps = n(clip.fps) || 30;
      const range: [number, number] = [player.range[0] / fps, player.range[1] / fps];
      const next = advance(player.time, delta * Number(player.speed), range, player.loop);
      set({ player: { ...player, time: next.time, playing: next.playing } });
    },

    revert: () => set({ draft: get().saved, outside: false }),

    save: async () => {
      const { project, view, draft } = get();
      if (!project || !view || get().saving) return false;
      const notify = useAppStore.getState().notify;
      set({ saving: true });
      try {
        const next = await commands.motionSave(project, draft.states, draft.clipsOff);
        if (get().project === project) take(next);
        // The top bar and the Pipeline view show the scene budget too.
        void usePipelineStore.getState().refresh(project);
        return true;
      } catch (error) {
        notify({ status: 'failed', title: 'Motion was not saved', detail: errorMessage(error) });
        return false;
      } finally {
        set({ saving: false });
      }
    },
  };
});

/** True when the draft has changes that are not saved. */
export function useMotionDirty(): boolean {
  return useMotionStore(s => !sameDraft(s.draft, s.saved));
}
