import type { Pivot } from '@/ipc/client';
import type { Vec3 } from '@/state/inspector';

/** One part as the stage shows it: its model, its pivot and its place. */
export interface StagePart {
  asset: string;
  /** The model's file on disk, absolute; `null` shows nothing. */
  file: string | null;
  pivot: Pivot;
  /** Where its pivot stands, metres, Y up; `null` where its model put it. */
  at: Vec3 | null;
  /** Degrees about the up axis. */
  turn: number;
}

/** What the gizmo does: move a part, or turn it about its pivot. */
export type GizmoMode = 'translate' | 'rotate';

export interface AssembleStageProps {
  parts: StagePart[];
  /** The part the gizmo is on. */
  selected: string | null;
  mode: GizmoMode;
  /** Snapping steps: metres for a move, degrees for a turn; `null` is free. */
  snap: { move: number | null; turn: number | null };
  onSelect: (asset: string | null) => void;
  /** A part's new place and turn, as the gizmo or its load sets them. */
  onPlace: (asset: string, at: Vec3, turn: number) => void;
  onError: (message: string) => void;
  className?: string;
}
