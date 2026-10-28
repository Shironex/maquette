import { useMemo, useState } from 'react';
import type { GizmoMode, StagePart } from '@/features/viewer/AssembleStage';
import { projectFile } from '@/state/board';
import type { Vec3 } from '@/state/inspector';
import { SNAP } from '@/state/layout';
import type { DraftPart } from '@/state/split';
import { useAppStore } from '@/state/use-app-store';

/** Snapping steps for a move, metres, and for a turn, degrees; `off` is free. */
export const MOVE_SNAPS = [
  { value: 'off', label: 'Free' },
  { value: '0.01', label: 'Snap 1 cm' },
  { value: String(SNAP.move), label: 'Snap 5 cm' },
  { value: '0.1', label: 'Snap 10 cm' },
];
export const TURN_SNAPS = [
  { value: 'off', label: 'Free' },
  { value: String(SNAP.rotate), label: 'Snap 15°' },
  { value: '45', label: 'Snap 45°' },
  { value: '90', label: 'Snap 90°' },
];

const step = (value: string) => (value === 'off' ? null : Number(value));

/**
 * The parts in the viewer: each part's model file (its chosen version or
 * the newest), the part the gizmo is on, the gizmo's mode and its snapping.
 * Places and turns go back to the draft as the gizmo moves them.
 */
export function useAssembleViewer(
  parts: readonly DraftPart[],
  onPlace: (index: number, patch: Partial<DraftPart>) => void
) {
  const workspace = useAppStore(s => s.workspace);
  const [chosen, setChosen] = useState<string | null>(null);
  const [mode, setMode] = useState<GizmoMode>('translate');
  const [moveSnap, setMoveSnap] = useState(String(SNAP.move));
  const [turnSnap, setTurnSnap] = useState(String(SNAP.rotate));
  const [error, setError] = useState<string | null>(null);

  const stage = useMemo<StagePart[]>(
    () =>
      parts.map(p => {
        const asset = workspace?.project?.assets?.find(a => a.id === p.asset);
        const version = p.version
          ? asset?.versions.find(v => v.version === p.version)
          : asset?.versions.at(-1);
        return {
          asset: p.asset,
          file: workspace && version ? projectFile(workspace, version.file) : null,
          pivot: p.pivot,
          at: p.at ? (p.at.map(v => v ?? 0) as Vec3) : null,
          turn: p.turn ?? 0,
        };
      }),
    [parts, workspace]
  );

  // The gizmo stays on a part: the one picked while it is in, else the first.
  const selected = parts.some(p => p.asset === chosen) ? chosen : (parts[0]?.asset ?? null);
  const place = (asset: string, at: Vec3, turn: number) => {
    const index = parts.findIndex(p => p.asset === asset);
    if (index >= 0) onPlace(index, { at, turn });
  };
  const snapValue = mode === 'translate' ? moveSnap : turnSnap;
  const setSnap = mode === 'translate' ? setMoveSnap : setTurnSnap;

  return {
    stage,
    /** The files shown, so the canvas tries again when they change. */
    files: stage.map(p => p.file).join('|'),
    error,
    setError,
    selected,
    select: (asset: string | null) => asset && setChosen(asset),
    mode,
    setMode,
    snap: { move: step(moveSnap), turn: step(turnSnap) },
    snapValue,
    snapOptions: mode === 'translate' ? MOVE_SNAPS : TURN_SNAPS,
    setSnap,
    place,
  };
}
