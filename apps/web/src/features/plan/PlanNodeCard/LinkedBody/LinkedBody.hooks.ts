import type { PlanNode } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { openPlanLink } from '@/state/open-plan-link';
import { assetView, directionView } from '@/state/plan-links';
import { usePlanLinksStore } from '@/state/use-plan-links-store';
import { usePlansStore } from '@/state/use-plans-store';

/**
 * An asset or Lab direction node's live view: what the last read of the
 * project said about its asset or direction, its picture and its next step.
 */
export function useLinkedBody(node: PlanNode) {
  const asset = usePlanLinksStore(s => (node.asset ? s.assets[node.asset] : undefined));
  const direction = usePlanLinksStore(s =>
    node.direction ? s.directions[node.direction] : undefined
  );
  const project = usePlansStore(s => s.project);
  const view = node.kind === 'asset' ? assetView(asset) : directionView(direction);
  return {
    view,
    src: fileUrl(view.picture),
    go: () => {
      if (project) openPlanLink(project, node, view.action.to, asset?.version ?? null);
    },
  };
}
