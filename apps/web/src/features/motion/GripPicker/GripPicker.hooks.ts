import { useEffect, useState } from 'react';
import type { GripHands, LibraryGripSummary } from '@/ipc/client';
import { LIMITS } from '@/state/recipe-check';
import { freeId } from '@/state/recipe-draft';
import { GRIP_EASE_FRAMES, defaultHands, layerOf, poseFor } from '@/state/recipe-grips';
import { useAppStore } from '@/state/use-app-store';
import { useGripLibraryStore } from '@/state/use-grip-library-store';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { useRecipeTimeline } from '@/state/use-recipe-timeline';

/** Add a grip's grid: the grip library, read when it opens, and the grip picked in it. */
export function useGripPicker() {
  const list = useGripLibraryStore(s => s.list);
  const error = useGripLibraryStore(s => s.error);
  const filter = useGripLibraryStore(s => s.filter);
  const load = useGripLibraryStore(s => s.load);
  const root = useAppStore(s => s.workspace?.root);
  const pickedId = useRecipeEditorStore(s => (s.picked?.what === 'grips' ? s.picked.id : ''));
  useEffect(() => {
    void load();
  }, [load, root]);
  const { pick } = useRecipeEditorStore.getState();
  const usable = list?.filter(g => g.hands && !g.problem) ?? [];
  const grip = usable.find(g => g.id === pickedId) ?? usable[0] ?? null;
  return {
    list,
    error,
    filter,
    setFilter: useGripLibraryStore.getState().setFilter,
    grip,
    select: (id: string) => pick({ what: 'grips', id }),
    close: () => pick(null),
    retry: () => void load(),
  };
}

const FINGER = /(Thumb|Index|Middle|Ring|Little)/;

/**
 * The picked grip's settings before it is added: the hands, the blank it
 * holds, the frames it holds on and how many it eases in over; what it
 * poses, and whether the recipe already holds an older revision of it.
 */
export function useGripAside(grip: LibraryGripSummary) {
  const draft = useRecipeEditorStore(s => s.editing?.draft ?? null);
  const dt = useRecipeTimeline();
  const frames = dt?.timeline.frames ?? 0;
  const hands0 = grip.hands ? defaultHands({ hands: grip.hands }) : 'right';
  const [hands, setHands] = useState<GripHands>(hands0);
  const [target, setTarget] = useState<string | null>(null);
  const [from, setFrom] = useState(0);
  const [to, setTo] = useState<number | null>(null);
  const [easeIn, setEaseIn] = useState(GRIP_EASE_FRAMES);
  const last = Math.min(to ?? frames, frames);
  const shown = grip.hands ? poseFor({ hands: grip.hands, pose: grip.pose }, hands) : null;
  const layers = draft?.layers ?? [];
  const older = layers.find(
    l =>
      l.kind === 'grip' &&
      l.from?.entry === grip.id &&
      (!l.from.uid || l.from.uid === grip.uid) &&
      l.from.revision < grip.latest
  );
  const full = layers.length >= LIMITS.layers;
  const blocked = full
    ? `A recipe holds at most ${LIMITS.layers} changes`
    : !(from < last)
      ? 'It has to start before it ends'
      : undefined;
  const blanks = draft?.blanks ?? [];
  return {
    hands,
    setHands,
    target,
    setTarget,
    targets: [
      { value: '', label: 'Nothing, the hands only' },
      ...blanks.map(b => ({ value: b.id, label: `${b.label} (a blank)` })),
    ],
    frames,
    from,
    setFrom,
    to: last,
    setTo,
    easeIn,
    setEaseIn,
    shown,
    fingers: shown ? Object.keys(shown.pose).filter(b => FINGER.test(b)).length : 0,
    older: older?.from ? { layer: older.id, revision: older.from.revision } : null,
    blocked,
    add: () => {
      if (!draft || !dt || !grip.hands || blocked) return;
      const id = freeId(
        'l',
        layers.map(l => l.id)
      );
      const from0 = { entry: grip.id, uid: grip.uid || null, revision: grip.latest };
      const made = layerOf(id, { hands: grip.hands, pose: grip.pose }, from0, hands);
      const settings = {
        ...made.settings,
        target,
        start: dt.toShare(from),
        end: dt.toShare(last),
        easeIn,
      };
      const { edit, pick } = useRecipeEditorStore.getState();
      edit(d => ({ ...d, layers: [...d.layers, { ...made, settings }] }));
      pick({ what: 'layer', id });
    },
  };
}
