import type { ScenePath } from '@/ipc/client';
import type { Vec3 } from '@/state/inspector';
import { addPoint, movePoint, pointsOf, removePoint, snapPoint } from '@/state/paths';
import { usePathsStore } from '@/state/use-paths-store';

/** Where a point added at the end goes: half a metre on, the way the last run points. */
export function nextPoint(path: ScenePath): Vec3 {
  const points = pointsOf(path);
  const last = points.at(-1) ?? [0, 0, 0];
  const before = points.at(-2) ?? [last[0] - 1, last[1], last[2]];
  const run = last.map((v, i) => v - before[i]!) as Vec3;
  const length = Math.hypot(...run) || 1;
  return snapPoint(last.map((v, i) => v + (run[i]! / length) * 0.5) as Vec3, false);
}

/** The table's edits: pick, move, add and remove points. */
export function usePointsTable(path: ScenePath) {
  const selection = usePathsStore(s => s.selection);
  const select = usePathsStore(s => s.select);
  const edit = usePathsStore(s => s.edit);
  const picked = selection?.path === path.id ? selection.point : null;
  return {
    picked,
    pick: (point: number) => select({ path: path.id, point }),
    move: (index: number, point: Vec3) => edit(d => movePoint(d, path.id, index, point)),
    add: () => {
      const s = usePathsStore.getState();
      const { draft, index } = addPoint(s.draft, path.id, nextPoint(path));
      if (index >= 0) edit(() => draft, { path: path.id, point: index });
    },
    remove: (index: number) =>
      edit(d => removePoint(d, path.id, index), { path: path.id, point: null }),
  };
}
