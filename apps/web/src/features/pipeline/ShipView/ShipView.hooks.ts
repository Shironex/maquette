import { useEffect, useState } from 'react';
import type { Approval, ShipFile } from '@/ipc/client';
import { formatBytes, formatK, homePath } from '@/lib/format';
import { shipGroups } from '@/state/game';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { useGameStore } from '@/state/use-game-store';
import { usePipelineStore } from '@/state/use-pipeline-store';

/** The Ship approval waiting for this project, if any. */
export function useShipApproval(project: string | undefined): Approval | null {
  return useAgentsStore(
    s => s.status?.approvals.find(a => a.kind === 'ship' && a.project === project) ?? null
  );
}

/** "props/" and "brazier.glb" for `props/brazier.glb`. */
export function splitPath(path: string): [string, string] {
  const at = path.lastIndexOf('/');
  return at < 0 ? ['', path] : [path.slice(0, at + 1), path.slice(at + 1)];
}

const size = (bytes: number | null) => (bytes ? formatBytes(bytes) : null);
const tris = (n: number | null) => (n === null ? null : `${formatK(n)} tris`);

/** The size and model cells of one row: before to after when it is replaced. */
export function shipCells(file: ShipFile): { size: string; model: string } {
  if (file.change === 'removed') {
    return { size: size(file.beforeBytes) ?? '', model: tris(file.beforeTriangles) ?? '' };
  }
  if (file.change === 'replaced') {
    const before = size(file.beforeBytes);
    const after = size(file.bytes);
    const tb = file.beforeTriangles;
    const ta = file.triangles;
    return {
      size: before && after ? `${before} → ${after}` : (after ?? ''),
      model: tb !== null && ta !== null && tb !== ta ? `${formatK(tb)} → ${formatK(ta)} tris` : '',
    };
  }
  return {
    size: [tris(file.triangles), size(file.bytes)].filter(Boolean).join(' · '),
    model: file.asset ?? '',
  };
}

/** The Ship plan of the open game project, and the approval that waits for it. */
export function useShipView(project: string) {
  const plan = useGameStore(s => s.plan);
  const loadPlan = useGameStore(s => s.loadPlan);
  const approval = useShipApproval(project);
  const decide = useAgentsStore(s => s.decide);
  const deciding = useAgentsStore(s => (approval ? s.deciding === approval.id : false));
  const name = useAppStore(s => s.workspace?.project?.name ?? project);
  const home = useAppStore(s => s.boot?.home);
  const [showAll, setShowAll] = useState(false);
  const runId = approval?.runId ?? null;
  useEffect(() => {
    void loadPlan(project);
  }, [project, runId, loadPlan]);
  const answer = async (approve: boolean) => {
    if (!approval) return;
    await decide(approval.id, approve);
    void usePipelineStore.getState().refresh(project);
    void loadPlan(project);
  };
  const groups = plan ? shipGroups(plan) : null;
  const repo = plan?.repo ? homePath(plan.repo, home) : null;
  return { plan, groups, approval, deciding, answer, name, repo, showAll, setShowAll };
}
