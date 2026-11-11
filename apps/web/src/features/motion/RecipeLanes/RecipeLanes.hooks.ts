import { useEffect, useMemo } from 'react';
import type { RecipeLayer } from '@/ipc/client';
import type { RecipePick } from '@/state/recipe-editor.types';
import {
  blendBlocks,
  clipBlocks,
  rulerMarks,
  statusOf,
  timeBlocks,
  trimBlocks,
} from '@/state/recipe-lanes';
import { useGripLibraryStore } from '@/state/use-grip-library-store';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { pinRows } from '@/state/recipe-pins';
import { useRecipeTimeline } from '@/state/use-recipe-timeline';

/**
 * The open recipe's lanes on the bake's frames: the base edits (clips with
 * their blends, trims, retimes, holds, the mirror) and a bar per change,
 * and which of them is picked.
 */
export function useRecipeLanes() {
  const dt = useRecipeTimeline();
  const editing = useRecipeEditorStore(s => s.editing);
  const picked = useRecipeEditorStore(s => s.picked);
  const grips = useGripLibraryStore(s => s.list);
  const draft = editing?.draft ?? null;
  const holdsGrip = draft?.layers.some(l => l.kind === 'grip') ?? false;
  useEffect(() => {
    // The grip bars name their library grip, so read the library once a recipe holds one.
    if (holdsGrip && !useGripLibraryStore.getState().list)
      void useGripLibraryStore.getState().load();
  }, [holdsGrip]);
  const doc = editing?.doc ?? null;
  const lanes = useMemo(() => {
    if (!dt || !draft || !doc) return null;
    const t = dt.timeline;
    const statuses = draft.clips.map((_, i) => statusOf(doc.base, doc.joined, draft.kept[i]));
    return {
      clips: clipBlocks(t, draft.clips).map((b, i) => ({ ...b, status: statuses[i] ?? 'found' })),
      blends: blendBlocks(t),
      trims: trimBlocks(t, draft.clips),
      retimes: timeBlocks(t, draft.time, ['retime']),
      holds: timeBlocks(t, draft.time, ['freeze', 'hold-over']),
      marks: rulerMarks(t.frames, t.fps),
    };
  }, [dt, draft, doc]);
  const pins = useMemo(() => (dt && draft ? pinRows(draft.layers, dt.toFrame) : []), [dt, draft]);
  const { pick } = useRecipeEditorStore.getState();
  const isPicked = (what: NonNullable<RecipePick>['what'], id = '') =>
    picked?.what === what && picked.id === id;
  return {
    dt,
    lanes,
    /** Every change but the pins, which have a lane of their own. */
    layers: (draft?.layers ?? []).filter(l => l.kind !== 'pin'),
    pins,
    mirror: draft?.mirror ?? false,
    /** A grip's bar: "Scythe, staff grip r1" from its library link, else "Grip". */
    gripLabel: (layer: RecipeLayer) => {
      const from = layer.from;
      if (!from) return 'Grip';
      return `${grips?.find(g => g.id === from.entry)?.name ?? from.entry} r${from.revision}`;
    },
    isPicked,
    /** Shows the settings of `what`, or hides them when they show. */
    toggle: (what: NonNullable<RecipePick>['what'], id = '') =>
      pick(isPicked(what, id) ? null : { what, id }),
  };
}
