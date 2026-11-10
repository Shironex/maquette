import { useEffect, useEffectEvent, useMemo, useState } from 'react';
import type { AnimationItem, RecipeBase } from '@/ipc/client';
import type { JoinedClip, RecipeMarker } from '@/features/viewer/RecipePreview';
import { blankFilled, num } from '@/state/recipe-draft';
import type { ReachCheck } from '@/state/recipe-review';
import { layoutSpot } from '@/features/viewer/recipe-layout';
import { useAnimationsStore } from '@/state/use-animations-store';
import { useLayoutStore } from '@/state/use-layout-store';
import { useMotionStore } from '@/state/use-motion-store';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { usePoseKeysStore } from '@/state/use-pose-keys-store';
import { clipFps, clipsOf } from '@/state/recipe-timeline';
import { useRecipeTimeline } from '@/state/use-recipe-timeline';
import { usePreviewGrips } from './use-preview-grips';
import { usePreviewPins } from './use-preview-pins';

/** The library clip a recipe's clip names: the same name and frames, the same keys first. */
export function libraryClip(items: readonly AnimationItem[], clip: RecipeBase) {
  const name = clip.name.trim().toLowerCase();
  const same = items.filter(
    i => i.clip.frames === clip.frames && i.clip.name.trim().toLowerCase() === name
  );
  return same.find(i => i.clip.hash === clip.hash) ?? same[0] ?? null;
}

/**
 * The preview's model, clips and markers, its playhead (a frame of the
 * bake, looping while it plays) and the live checks at each reach's end.
 */
export function useRecipePlayer() {
  const editing = useRecipeEditorStore(s => s.editing);
  const view = useMotionStore(s => s.view);
  const asset = useMotionStore(s => s.asset);
  const library = useAnimationsStore(s => s.view);
  const [position, setPosition] = useState(0);
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
  const dt = useRecipeTimeline();
  const frames = dt?.timeline.frames ?? 0;
  const fps = dt?.timeline.fps ?? 30;
  const joined = useMemo(() => {
    if (!dt || dt.plain) return null;
    const clips: JoinedClip[] = clipsOf(dt.recipe).map(c => {
      const found = libraryClip(library?.items ?? [], c);
      return { id: found?.clip.id ?? null, file: found?.file ?? null, fps: clipFps(c) };
    });
    return { joined: clips, timeline: dt.timeline };
  }, [dt, library]);
  const scrub = (frame: number) => {
    setPlaying(false);
    setPosition(Math.min(Math.max(frame, 0), frames));
  };
  // The pose keys editor keys the frame on show and moves the playhead to a key.
  const shown = Math.round(position);
  useEffect(() => usePoseKeysStore.getState().setFrame(shown), [shown]);
  const scrubTo = useEffectEvent(scrub);
  useEffect(() => {
    usePoseKeysStore.getState().setSeek(frame => scrubTo(frame));
    return () => usePoseKeysStore.getState().setSeek(null);
  }, []);
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
    share: frames > 0 ? position / frames : 0,
    position,
    frame: Math.round(position),
    joined,
    playing,
    /** The clips pose keys stand on. */
    keysOn: dt?.recipe ?? null,
    toggle: () => setPlaying(p => !p),
    scrub,
    tick: (delta: number) => setPosition(p => (frames > 0 ? (p + delta * fps) % frames : 0)),
    checks,
    setChecks,
    pins: usePreviewPins(dt, editing?.draft.layers),
    grips: usePreviewGrips(dt, editing?.draft.layers),
  };
}
