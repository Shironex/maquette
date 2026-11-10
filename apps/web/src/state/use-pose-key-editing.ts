/**
 * Editing the picked pose key change: the key under the playhead (a frame
 * of one clip, so it moves with that clip), the picked bone's turn there,
 * and every edit the form, the lanes, the gizmo and the shortcuts make.
 * Off the playhead's key, a bone shows the turn the keys blend to there,
 * so keying it holds the pose where it is.
 */
import { useRef } from 'react';
import type { KeyEase, LayerMode, RecipeLayer } from '@/ipc/client';
import { boneWords } from './recipe-bones';
import { num } from './recipe-draft';
import { limitLine, overLimits } from './joint-limits';
import {
  keyAt,
  mirrorPose,
  moveKey,
  removeKey,
  setEase,
  setPose,
  setTurn,
  type KeySpot,
} from './recipe-key-edits';
import {
  clipFrameAt,
  keyFrame,
  keyedBones,
  layerTurns,
  shareAt,
  turnOf,
  type Turn,
} from './recipe-keys';
import { useRecipeEditorStore } from './use-recipe-editor-store';
import { usePoseKeysStore } from './use-pose-keys-store';
import { useRecipeTimeline } from './use-recipe-timeline';

const ZERO: Turn = [0, 0, 0];

/** A key on the ruler: where it sits, the bake frame it shows on (null when a trim cuts it out). */
export interface KeyMark {
  spot: KeySpot;
  frame: number | null;
  ease: KeyEase;
  bones: string[];
}

/** The pose key change `id` (or the picked one when it is a pose key change) and its edits. */
export function usePoseKeyEditing(id?: string) {
  const draft = useRecipeEditorStore(s => s.editing?.draft ?? null);
  const picked = useRecipeEditorStore(s => s.picked);
  const dt = useRecipeTimeline();
  const s = usePoseKeysStore();
  const layerId = id ?? (picked?.what === 'layer' ? picked.id : null);
  const found = draft?.layers.find(l => l.id === layerId) ?? null;
  const layer = found?.kind === 'pose' ? found : null;
  /** The key's turn when a gizmo drag began. */
  const dragFrom = useRef<Turn | null>(null);

  const frame = Math.round(s.frame);
  const spot: KeySpot = dt ? clipFrameAt(dt.timeline, frame) : { clip: 0, frame };
  const here = layer ? keyAt(layer, spot) : null;
  const blended = (bone: string): Turn => {
    if (!layer || !dt) return ZERO;
    return layerTurns(dt.recipe, layer, shareAt(dt.recipe, dt.timeline, frame)).get(bone) ?? ZERO;
  };
  const turnAt = (bone: string): Turn => (here ? turnOf(here, bone) : null) ?? blended(bone);
  const turn = turnAt(s.bone);
  const keyedHere = Boolean(here && here.pose[s.bone]);

  const put = (next: (layer: RecipeLayer) => RecipeLayer) => {
    if (!layer) return;
    useRecipeEditorStore
      .getState()
      .edit(d => ({ ...d, layers: d.layers.map(l => (l.id === layer.id ? next(l) : l)) }));
  };

  const marks: KeyMark[] =
    layer && dt
      ? (layer.keys ?? []).map(k => {
          const at = { clip: k.clip ?? 0, frame: k.frame };
          return {
            spot: at,
            frame: keyFrame(dt.timeline, at.clip, at.frame),
            ease: k.ease ?? 'smooth',
            bones: Object.keys(k.pose),
          };
        })
      : [];
  const shown = marks.flatMap(m => (m.frame === null ? [] : [m.frame])).sort((a, b) => a - b);
  const seek = (to: number) => {
    s.setFrame(to);
    s.seek?.(to);
  };

  const name = boneWords(s.bone);
  return {
    layer,
    dt,
    bone: s.bone,
    space: s.space,
    turning: s.turning,
    frame,
    spot,
    here,
    keyedHere,
    turn,
    marks,
    bones: layer ? keyedBones(layer) : [],
    ease: here?.ease ?? 'smooth',
    mode: (layer?.mode ?? 'add') as LayerMode,
    weight: num(layer?.settings.weight, 1),
    copied: s.copied,
    hits: overLimits(s.bone, turn),
    limits: limitLine(s.bone, turn, name),
    /** Keys with no frame on show: a trim cuts them out. */
    hidden: marks.filter(m => m.frame === null).length,

    pickBone: s.pickBone,
    setSpace: s.setSpace,
    toggleTurning: () => s.setTurning(!s.turning),
    setAxis: (axis: number, degrees: number) => {
      const next = [...turn] as Turn;
      next[axis] = degrees;
      put(l => setTurn(l, spot, s.bone, next));
    },
    /** K: keys the picked bone where it stands now. */
    key: () => put(l => setTurn(l, spot, s.bone, turn)),
    setEase: (ease: KeyEase) =>
      put(l =>
        keyAt(l, spot) ? setEase(l, spot, ease) : setPose(l, spot, { [s.bone]: turn }, ease)
      ),
    setMode: (mode: LayerMode) => put(l => ({ ...l, mode })),
    setWeight: (weight: number) =>
      put(l => ({ ...l, settings: { ...l.settings, weight: Math.min(Math.max(weight, 0), 1) } })),
    /** Removes the picked bone's key here, or the whole key with `all`. */
    remove: (all = false) => put(l => removeKey(l, spot, all ? null : s.bone)),
    copy: () => s.copy(here ? here.pose : { [s.bone]: turn }),
    paste: () => {
      const pose = s.copied;
      if (pose) put(l => setPose(l, spot, pose, here?.ease ?? 'smooth'));
    },
    /** M: the key here (or the picked bone) as its mirror image, the other side's bones keyed. */
    mirror: () => {
      const pose = here ? here.pose : { [s.bone]: turn };
      put(l => setPose(removeKey(l, spot, null), spot, mirrorPose(pose), here?.ease ?? 'smooth'));
    },
    /** Moves (or with `copy`, copies) the key at `from` to bake frame `to`; only `bones` when given. */
    move: (from: KeySpot, to: number, opts: { copy?: boolean; bones?: string[] } = {}) => {
      if (!dt) return;
      put(l => moveKey(l, from, clipFrameAt(dt.timeline, to), opts));
    },
    /** [ and ]: the playhead to the key before or after. */
    step: (way: -1 | 1) => {
      const next = way < 0 ? shown.filter(f => f < frame).pop() : shown.find(f => f > frame);
      if (next !== undefined) seek(next);
    },
    seek,
    /** A gizmo drag: turns the picked bone on top of the turn it had when the drag began. */
    gizmoTurn: (phase: 'start' | 'move' | 'end', next: (from: Turn) => Turn) => {
      if (phase === 'start') dragFrom.current = turn;
      else if (phase === 'end') dragFrom.current = null;
      else if (dragFrom.current) {
        const from = dragFrom.current;
        put(l => setTurn(l, spot, s.bone, next(from)));
      }
    },
  };
}

export type PoseKeyEditing = ReturnType<typeof usePoseKeyEditing>;
