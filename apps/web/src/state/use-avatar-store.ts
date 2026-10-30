/**
 * The Avatar view: one character asset's VRM export. The spec is edited
 * as a draft and saved before a build; a build that frees cloth and hair
 * from the arms or changes the rest pose waits for the person's yes to
 * each, given here and nowhere else. Agents that
 * change or build an avatar raise `avatarChanged`, which reloads it.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type AvatarSpec_Serialize,
  type AvatarView_Serialize,
  type Elbows,
  type Fingers,
  type VrmMeta_Serialize,
} from '@/ipc/client';
import { buildNotice } from './avatar';
import { useAppStore } from './use-app-store';
import { useAgentsStore } from './use-agents-store';

/** The Avatar view's built tabs. */
export type AvatarTab = 'export' | 'preview';

interface AvatarStore {
  project: string | null;
  asset: string | null;
  /** The model version shown; null is the newest. */
  version: number | null;
  view: AvatarView_Serialize | null;
  draft: AvatarSpec_Serialize | null;
  /** The draft differs from what is stored. */
  dirty: boolean;
  loading: boolean;
  error: string | null;
  saving: boolean;
  /** The bone whose joint is being saved. */
  savingBone: string | null;
  building: boolean;
  deciding: boolean;
  elbows: Elbows;
  fingers: Fingers;
  tab: AvatarTab;
  /** Opens the Avatar view on an asset, on one of its versions (the newest without). */
  open: (project: string, asset: string, version?: number | null) => void;
  select: (asset: string | null) => void;
  /** Shows another version of the open asset; the draft spec stays. */
  setVersion: (version: number | null) => void;
  load: () => Promise<void>;
  editMeta: (change: Partial<VrmMeta_Serialize>) => void;
  setMaterial: (material: AvatarSpec_Serialize['material']) => void;
  setElbows: (elbows: Elbows) => void;
  setFingers: (fingers: Fingers) => void;
  setTab: (tab: AvatarTab) => void;
  save: () => Promise<boolean>;
  /**
   * Maps a VRM bone by hand and saves it at once: a joint name, null to
   * leave the bone unmapped, undefined to map it by name again. The meta
   * draft stays as it is.
   */
  setBone: (bone: string, joint: string | null | undefined) => Promise<void>;
  build: () => Promise<void>;
  decide: (approve: boolean) => Promise<void>;
  /** The yes or no to the weights that free cloth and hair from the arms; a yes builds on. */
  decideReweight: (approve: boolean) => Promise<void>;
  /** An agent changed an avatar; reloads it when it is the one open. */
  onChanged: (project: string, asset: string) => void;
}

const notify = (status: 'done' | 'failed' | 'info', title: string, detail?: string) =>
  useAppStore.getState().notify({ status, title, detail });

export const useAvatarStore = create<AvatarStore>((set, get) => ({
  project: null,
  asset: null,
  version: null,
  view: null,
  draft: null,
  dirty: false,
  loading: false,
  error: null,
  saving: false,
  savingBone: null,
  building: false,
  deciding: false,
  elbows: 'straighten',
  fingers: 'straighten',
  tab: 'export',

  open: (project, asset, version = null) => {
    set({ project, asset, version, view: null, draft: null, dirty: false, error: null });
    useAppStore.getState().setView('avatar');
    void get().load();
  },

  select: asset => {
    if (asset === get().asset) return;
    set({
      asset,
      version: null,
      view: null,
      draft: null,
      dirty: false,
      error: null,
      elbows: 'straighten',
      fingers: 'straighten',
    });
    void get().load();
  },

  setVersion: version => {
    if (version === get().version) return;
    set({ version, view: null, error: null });
    void get().load();
  },

  load: async () => {
    const { project, asset, version } = get();
    if (!project || !asset) return;
    set({ loading: true, error: null });
    try {
      const view = await commands.avatarGet(project, asset, version);
      if (get().asset !== asset || get().version !== version) return;
      // Keep what the person is typing; take the stored spec otherwise.
      set(s => ({ view, draft: s.dirty && s.draft ? s.draft : view.spec, loading: false }));
    } catch (error) {
      if (get().asset === asset && get().version === version)
        set({ loading: false, error: errorMessage(error) });
    }
  },

  editMeta: change => {
    const draft = get().draft;
    if (draft) set({ draft: { ...draft, meta: { ...draft.meta, ...change } }, dirty: true });
  },

  setMaterial: material => {
    const draft = get().draft;
    if (draft) set({ draft: { ...draft, material }, dirty: true });
  },

  setElbows: elbows => set({ elbows }),

  setFingers: fingers => set({ fingers }),

  setTab: tab => set({ tab }),

  save: async () => {
    const { project, asset, draft, version } = get();
    if (!project || !asset || !draft) return false;
    set({ saving: true });
    try {
      const view = await commands.avatarSave(project, asset, draft);
      // A save reads the newest version back; another one shown is read again.
      if (version === null) set({ view });
      set({ draft: view.spec, dirty: false, saving: false });
      if (version !== null) await get().load();
      return true;
    } catch (error) {
      set({ saving: false });
      notify('failed', 'Could not save the avatar', errorMessage(error));
      return false;
    }
  },

  setBone: async (bone, joint) => {
    const { project, asset, view } = get();
    if (!project || !asset || !view) return;
    const bones = { ...view.spec.bones };
    if (joint === undefined) delete bones[bone];
    else bones[bone] = joint;
    set({ savingBone: bone });
    try {
      const saved = await commands.avatarSave(project, asset, { ...view.spec, bones });
      set(s => ({
        draft: s.dirty && s.draft ? { ...s.draft, bones: saved.spec.bones } : saved.spec,
      }));
      await get().load();
    } catch (error) {
      notify('failed', 'Could not map the bone', errorMessage(error));
    } finally {
      set({ savingBone: null });
    }
  },

  build: async () => {
    const { project, asset, version, dirty, elbows, fingers } = get();
    if (!project || !asset) return;
    if (dirty && !(await get().save())) return;
    set({ building: true });
    try {
      const built = await commands.avatarBuild(
        project,
        asset,
        version,
        elbows,
        null,
        fingers,
        null
      );
      const { title, detail } = buildNotice(built);
      notify(built.status === 'written' ? 'done' : 'info', title, detail);
    } catch (error) {
      notify('failed', 'Could not build the avatar', errorMessage(error));
    } finally {
      set({ building: false });
      await get().load();
    }
  },

  decide: async approve => {
    const { project, asset, version } = get();
    if (!project || !asset) return;
    set({ deciding: true });
    try {
      await commands.avatarDecide(project, asset, version, approve);
      void useAgentsStore.getState().load();
      if (approve) {
        set({ deciding: false });
        await get().build();
        return;
      }
      notify('info', 'Rest pose turned down', 'Nothing is written from it.');
    } catch (error) {
      notify('failed', 'Could not answer the rest pose', errorMessage(error));
    } finally {
      set({ deciding: false });
      await get().load();
    }
  },

  decideReweight: async approve => {
    const { project, asset, version } = get();
    if (!project || !asset) return;
    set({ deciding: true });
    try {
      await commands.avatarDecideReweight(project, asset, version, approve);
      void useAgentsStore.getState().load();
      if (approve) {
        set({ deciding: false });
        await get().build();
        return;
      }
      notify(
        'info',
        'New weights turned down',
        'Build again to lift the arms from the model as it is.'
      );
    } catch (error) {
      notify('failed', 'Could not answer the new weights', errorMessage(error));
    } finally {
      set({ deciding: false });
      await get().load();
    }
  },

  onChanged: (project, asset) => {
    const s = get();
    if (s.project === project && s.asset === asset) void s.load();
  },
}));
