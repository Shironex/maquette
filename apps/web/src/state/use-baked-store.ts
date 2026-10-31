/**
 * What the open project took from the animation library, for Motion: which
 * of a model's clips a bake put there, from which library clip, and what
 * the last bake left out. Read again when the project's models change.
 */
import { create } from 'zustand';
import {
  commands,
  type AssetAnimations,
  type BakedClip,
  type ProjectAnimations,
} from '@/ipc/client';

interface BakedStore {
  project: string | null;
  record: ProjectAnimations | null;
  load: (project: string) => Promise<void>;
}

export const useBakedStore = create<BakedStore>((set, get) => ({
  project: null,
  record: null,
  load: async project => {
    if (get().project !== project) set({ project, record: null });
    try {
      const record = await commands.animationsOfProject(project);
      if (get().project === project) set({ record });
    } catch {
      // Motion still shows the clips, ungrouped, without the record.
      if (get().project === project) set({ record: null });
    }
  },
}));

const NOTHING: AssetAnimations = {
  map: [],
  follow: [],
  baked: [],
  review: null,
  reviewVersion: null,
};

/** One model's entry in the record. */
export const useBakedOf = (asset: string | null | undefined): AssetAnimations =>
  useBakedStore(s => (asset && s.record?.assets?.[asset]) || NOTHING);

/** The bake that put a clip of this name on the model. */
export const bakedClip = (entry: AssetAnimations, clip: string | null | undefined) =>
  (entry.baked ?? []).find((b: BakedClip) => b.setting.name === clip) ?? null;
