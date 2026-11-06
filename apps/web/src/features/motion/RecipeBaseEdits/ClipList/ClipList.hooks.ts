import { useState } from 'react';
import type { BlendWindow, RecipeBase } from '@/ipc/client';
import { checkDraft } from '@/state/recipe-check';
import { moveClip, patchClip, removeClip } from '@/state/recipe-clips';
import { statusOf } from '@/state/recipe-lanes';
import { DEFAULT_BLEND, V2_LIMITS, blendMost, keptFrames } from '@/state/recipe-v2-check';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { useRecipesStore } from '@/state/use-recipes-store';

/**
 * The open recipe's clips in order, with how each stands in the library,
 * the blend into each joined clip, and moving, taking out and adding one.
 */
export function useClipList() {
  const editing = useRecipeEditorStore(s => s.editing);
  const [problem, setProblem] = useState<string | null>(null);
  const draft = editing?.draft ?? null;
  const doc = editing?.doc ?? null;
  const { edit } = useRecipeEditorStore.getState();
  const problems = draft ? checkDraft(draft) : null;
  const clips = (draft?.clips ?? []).map((clip: RecipeBase, i) => ({
    clip,
    index: i,
    kept: keptFrames(clip),
    status: doc && draft ? statusOf(doc.base, doc.joined, draft.kept[i]) : 'found',
    uses: draft?.time.filter(t => t.clip === i).length ?? 0,
    most: draft ? blendMost(draft.clips, i) : 0,
    problem: problems?.clips[i] ?? null,
  }));
  const count = clips.length;
  return {
    clips,
    problem,
    general: problems?.general ?? [],
    canAdd: count < V2_LIMITS.clips && !draft?.clips[0]?.rest,
    move: (from: number, to: number) => {
      setProblem(null);
      edit(d => moveClip(d, from, to));
    },
    remove: (index: number) => {
      if (!draft) return;
      const out = removeClip(draft, index);
      if ('problem' in out) {
        setProblem(out.problem);
        return;
      }
      setProblem(null);
      edit(() => out.draft);
    },
    setBlend: (index: number, patch: Partial<BlendWindow>) =>
      edit(d => {
        const was = d.clips[index]?.blend ?? DEFAULT_BLEND;
        return patchClip(d, index, { blend: { ...was, ...patch } });
      }),
    add: () => {
      if (editing) useRecipesStore.getState().openPicker(editing.project, true);
    },
  };
}
