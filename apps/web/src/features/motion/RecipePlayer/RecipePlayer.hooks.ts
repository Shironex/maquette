import { useEffect, useMemo, useState } from 'react';
import type { RecipeMarker } from '@/features/viewer/RecipePreview';
import { blankFilled, num } from '@/state/recipe-draft';
import type { ReachCheck } from '@/state/recipe-review';
import { layoutSpot } from '@/features/viewer/recipe-layout';
import { useAnimationsStore } from '@/state/use-animations-store';
import { useLayoutStore } from '@/state/use-layout-store';
import { useMotionStore } from '@/state/use-motion-store';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';

/**
 * The preview's model, clip and markers, its playhead (a share of the clip,
 * looping while it plays) and the live checks at each reach's end.
 */
export function useRecipePlayer() {
  const editing = useRecipeEditorStore(s => s.editing);
  const view = useMotionStore(s => s.view);
  const asset = useMotionStore(s => s.asset);
  const library = useAnimationsStore(s => s.view);
  const [share, setShare] = useState(0);
  const [playing, setPlaying] = useState(true);
  const [checks, setChecks] = useState<ReachCheck[]>([]);
  useEffect(() => {
    if (!useAnimationsStore.getState().view) void useAnimationsStore.getState().load();
  }, []);
  const rigged = view?.models.filter(m => m.rig && !m.error) ?? [];
  const model = rigged.find(m => m.asset === asset) ?? rigged[0] ?? null;
  const base = editing?.doc.base;
  const rest = base?.status === 'rest';
  const item = rest ? null : (library?.items.find(i => i.clip.id === base?.clip) ?? null);
  const length = num(editing?.doc.recipe.base.length, 1) || 1;
  const frames = editing?.doc.recipe.base.frames ?? 0;
  const blanks = editing?.draft.blanks;
  const project = editing?.project ?? null;
  const layout = useLayoutStore(s => (project && s.project === project ? s.view : null));
  useEffect(() => {
    const store = useLayoutStore.getState();
    if (project && (store.project !== project || !store.view)) void store.load(project);
  }, [project]);
  const markers = useMemo<RecipeMarker[]>(
    () =>
      (blanks ?? []).filter(blankFilled).map(b => ({
        id: b.id,
        kind: b.kind,
        point: b.point ? [num(b.point[0]), num(b.point[1]), num(b.point[2])] : null,
        at:
          b.kind === 'prop' && b.prop && layout && model
            ? layoutSpot(layout, model.asset, b.prop)
            : null,
      })),
    [blanks, layout, model]
  );
  return {
    project,
    model: model ? { asset: model.asset, name: model.name, file: model.file } : null,
    clip: rest
      ? ('rest' as const)
      : item
        ? { id: item.clip.id, name: item.clip.name, file: item.file }
        : null,
    layers: editing?.draft.layers ?? [],
    markers,
    blanks: blanks ?? [],
    frames,
    share,
    frame: Math.round(share * frames),
    playing,
    toggle: () => setPlaying(p => !p),
    scrub: (frame: number) => {
      setPlaying(false);
      setShare(frames > 0 ? Math.min(Math.max(frame / frames, 0), 1) : 0);
    },
    tick: (delta: number) => setShare(s => (s + delta / length) % 1),
    checks,
    setChecks,
  };
}
