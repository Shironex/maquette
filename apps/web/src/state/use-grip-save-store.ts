/**
 * The save a grip sheet: the hand pose a recipe's pose key change holds at
 * one frame, saved to the grip library as a new grip or the next revision
 * of one. The pose is read from the editor's draft while that recipe is
 * open there (so unsaved keys count), else from the file. The sheet plans
 * after every choice, so the person reads the file and what changes before
 * anything is written, and Save sends back the version the plan read: a
 * grip that moved since is refused (`moved`), read again, and the person
 * looks before saving. A save always adds a revision. An agent's proposal
 * opens the same sheet from its approval card, preset to what it asked;
 * only the sheet's Save writes, and that closes the approval.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  isErrorPayload,
  type GripFile,
  type GripHands,
  type GripSaveAsk,
  type GripSavePlan,
  type GripSide,
  type RecipeFile,
} from '@/ipc/client';
import { recipeOfDraft } from './recipe-clips';
import { GRIP_VERSION, gripOf, keyedPose, type GripPose } from './recipe-grips';
import { openRecipesTab } from './open-recipes-tab';
import { useAppStore } from './use-app-store';
import { useGripLibraryStore } from './use-grip-library-store';
import { draftTimeline } from './use-recipe-timeline';
import { useRecipeEditorStore } from './use-recipe-editor-store';

export interface GripSheet {
  project: string;
  recipe: string;
  /** The pose key change it reads, and the bake frame. */
  layer: string;
  frame: number;
  /** Every hand, finger and arm bone the change holds at that frame; null until read. */
  pose: GripPose | null;
  name: string;
  weapon: string;
  hands: GripHands;
  /** The bones kept, by VRM name. */
  bones: string[];
  /** The library grip it becomes a revision of; null for a new grip. */
  entry: string | null;
  plan: GripSavePlan | null;
  planning: boolean;
  error: string | null;
  /** The library grip moved since the sheet planned, and was planned again. */
  moved: boolean;
  saving: boolean;
}

/** What the sheet opens with besides the pose: an agent's proposal. */
export type GripPreset = Partial<Pick<GripSheet, 'name' | 'weapon' | 'hands' | 'bones' | 'entry'>>;

/** The choices the person makes in the sheet. */
export type GripChoice = Partial<Pick<GripSheet, 'name' | 'weapon' | 'hands' | 'bones' | 'entry'>>;

interface GripSaveStore {
  sheet: GripSheet | null;
  /** Opens the sheet on pose key change `layer` of a project's recipe at bake frame `frame`. */
  open: (
    project: string,
    recipe: string,
    layer: string,
    frame: number,
    preset?: GripPreset
  ) => Promise<void>;
  /** From an agent's approval card: the project's Recipes tab, then the sheet as it asked. */
  show: (project: string, recipe: string, entry: string | null, ask: GripSaveAsk) => Promise<void>;
  choose: (choice: GripChoice) => Promise<void>;
  /** Saves as planned; true once saved. */
  save: () => Promise<boolean>;
  close: () => void;
}

const MOVED = 'moved';

/** The hands a proposal's holds read as. */
export const handsOfHolds = (hands: GripSaveAsk['hands']): GripHands =>
  hands.supports ? 'both' : hands.holds;

/**
 * The grip `sheet` would save: its kept bones for its hands, the right
 * hand holding a two hand grip; null while the pose is not read or turns
 * no bone on a side asked for.
 */
export function gripOfSheet(sheet: GripSheet): GripFile | null {
  if (!sheet.pose) return null;
  const kept: GripPose = Object.fromEntries(
    Object.entries(sheet.pose).filter(([bone]) => sheet.bones.includes(bone))
  );
  const made = gripOf(kept, sheet.hands, 'right' as GripSide);
  if (!made) return null;
  return {
    version: GRIP_VERSION,
    name: sheet.name.trim(),
    revision: 1,
    weapon: sheet.weapon,
    hands: made.hands,
    pose: made.pose,
  };
}

/** The recipe as the editor has it while open there, else as the file has it. */
async function recipeOf(project: string, recipe: string): Promise<RecipeFile> {
  const editing = useRecipeEditorStore.getState().editing;
  if (editing && editing.project === project && editing.id === recipe)
    return recipeOfDraft(editing.doc.recipe, editing.draft);
  return (await commands.recipeGet(project, recipe)).recipe;
}

/** Counts plans, so an older answer never replaces a newer one. */
let plans = 0;

export const useGripSaveStore = create<GripSaveStore>((set, get) => {
  const patch = (change: Partial<GripSheet>) => {
    const sheet = get().sheet;
    if (sheet) set({ sheet: { ...sheet, ...change } });
  };
  const plan = async (sheet: GripSheet) => {
    const read = ++plans;
    const grip = gripOfSheet(sheet);
    if (!grip?.name) {
      patch({ plan: null, planning: false, error: null });
      return;
    }
    patch({ planning: true, error: null });
    try {
      const next = await commands.gripLibraryPlanSave(grip, sheet.entry);
      if (read === plans) patch({ plan: next, planning: false });
    } catch (error) {
      if (read === plans) patch({ plan: null, planning: false, error: errorMessage(error) });
    }
  };
  return {
    sheet: null,

    open: async (project, recipe, layer, frame, preset = {}) => {
      const sheet: GripSheet = {
        project,
        recipe,
        layer,
        frame,
        pose: null,
        name: preset.name ?? '',
        weapon: preset.weapon ?? 'other',
        hands: preset.hands ?? 'both',
        bones: preset.bones ?? [],
        entry: preset.entry ?? null,
        plan: null,
        planning: true,
        error: null,
        moved: false,
        saving: false,
      };
      set({ sheet });
      const read = ++plans;
      try {
        const [file] = await Promise.all([
          recipeOf(project, recipe),
          useGripLibraryStore.getState().load(),
        ]);
        if (read !== plans) return;
        const change = file.layers.find(l => l.id === layer);
        const pose = change ? keyedPose(file, draftTimeline(file).timeline, change, frame) : null;
        if (!pose) {
          patch({
            planning: false,
            error: `${change ? 'The pose keys' : 'The change'} ${layer} turn no hand at frame ${frame}. Key a hand there first.`,
          });
          return;
        }
        const left = Object.keys(pose).some(b => b.startsWith('left'));
        const right = Object.keys(pose).some(b => b.startsWith('right'));
        const hands = preset.hands ?? (left && right ? 'both' : left ? 'left' : 'right');
        const entry = sheet.entry
          ? useGripLibraryStore.getState().list?.find(g => g.id === sheet.entry)
          : null;
        const ready: GripSheet = {
          ...sheet,
          pose,
          hands,
          bones: preset.bones?.filter(b => b in pose) ?? Object.keys(pose),
          name: sheet.name || entry?.name || file.name,
          weapon: preset.weapon ?? entry?.weapon ?? sheet.weapon,
        };
        set({ sheet: ready });
        await plan(ready);
      } catch (error) {
        if (read === plans) patch({ planning: false, error: errorMessage(error) });
      }
    },

    show: async (project, recipe, entry, ask) => {
      if (!(await openRecipesTab(project))) return;
      await get().open(project, recipe, ask.layer, ask.frame, {
        name: ask.name,
        weapon: ask.weapon,
        hands: handsOfHolds(ask.hands),
        bones: ask.bones,
        entry,
      });
    },

    choose: async choice => {
      const sheet = get().sheet;
      if (!sheet) return;
      const next = { ...sheet, ...choice, plan: null, moved: false };
      if (choice.entry !== undefined && choice.entry !== sheet.entry && choice.entry) {
        // A revision takes its library grip's name and weapon unless the person changes them.
        const entry = useGripLibraryStore.getState().list?.find(g => g.id === choice.entry);
        if (entry) Object.assign(next, { name: entry.name, weapon: entry.weapon });
      }
      set({ sheet: next });
      await plan(next);
    },

    save: async () => {
      const sheet = get().sheet;
      const planned = sheet?.plan;
      if (!sheet || !planned || planned.same || sheet.saving || sheet.planning) return false;
      patch({ saving: true, error: null });
      try {
        const saved = await commands.gripLibrarySave({
          grip: planned.grip,
          entry: planned.entry,
          expected: planned.entryVersion,
          source: {
            project: sheet.project,
            recipe: sheet.recipe,
            layer: sheet.layer,
            frame: sheet.frame,
          },
        });
        set({ sheet: null });
        useAppStore.getState().notify({
          status: 'done',
          title: planned.entry
            ? `Saved r${saved.revision} of ${saved.name}`
            : `Saved ${saved.name} to the grip library`,
        });
        await useGripLibraryStore.getState().load();
        return true;
      } catch (error) {
        if (isErrorPayload(error) && error.code === MOVED) {
          patch({ saving: false, moved: true });
          const now = get().sheet;
          if (now) await plan(now);
        } else patch({ saving: false, error: errorMessage(error) });
        return false;
      }
    },

    close: () => {
      plans += 1;
      set({ sheet: null });
    },
  };
});
