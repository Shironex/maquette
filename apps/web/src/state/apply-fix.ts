/**
 * A review's fix laid on the editor's draft: a wider blend into a clip, a
 * longer ease into or out of a freeze or hold over, or a keyed turn held
 * to its joint limit. Nothing is saved: the person sees it in the editor,
 * saves and bakes again. A fix whose clip, hold or key is no longer there
 * leaves the draft as it is.
 */
import type { RecipeFixEdit } from '@/ipc/client';
import type { RecipeDraft } from './recipe-draft';

const AXES = { x: 0, y: 1, z: 2 } as const;

/** `draft` with `edit` laid on it, or null when what it names is gone. */
export function applyFix(draft: RecipeDraft, edit: RecipeFixEdit): RecipeDraft | null {
  switch (edit.kind) {
    case 'blend': {
      const clip = draft.clips[edit.clip];
      if (!clip || edit.clip === 0) return null;
      const clips = draft.clips.map((c, i) =>
        i === edit.clip ? { ...c, blend: { ...(c.blend ?? {}), frames: edit.frames } } : c
      );
      return { ...draft, clips };
    }
    case 'hold-ease': {
      const at = draft.time.findIndex(
        t => t.kind !== 'retime' && t.clip === edit.clip && t.at === edit.at
      );
      if (at < 0) return null;
      const time = draft.time.map((t, i) =>
        i === at
          ? {
              ...t,
              ...(edit.easeIn != null ? { easeIn: edit.easeIn } : {}),
              ...(edit.easeOut != null ? { easeOut: edit.easeOut } : {}),
            }
          : t
      );
      return { ...draft, time };
    }
    case 'key-turn': {
      const axis = AXES[edit.axis as keyof typeof AXES];
      let found = false;
      const layers = draft.layers.map(layer => {
        if (layer.id !== edit.layer) return layer;
        const keys = (layer.keys ?? []).map(key => {
          const turn = key.pose[edit.bone];
          if ((key.clip ?? 0) !== edit.clip || key.frame !== edit.frame || !turn || axis == null)
            return key;
          found = true;
          const next = [...turn] as typeof turn;
          next[axis] = edit.degrees;
          return { ...key, pose: { ...key.pose, [edit.bone]: next } };
        });
        return { ...layer, keys };
      });
      return found ? { ...draft, layers } : null;
    }
  }
}
