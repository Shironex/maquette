/**
 * Where an asset or Lab direction node's next step goes: the Inspector on
 * the asset's newest version, the Inbox, the Lab (on the direction, for a
 * direction node) or the Board. The canvas stays open behind, so coming
 * back to Plan finds it as it was.
 */
import type { PlanNode } from '@/ipc/client';
import type { LinkAction } from './plan-links';
import { useAppStore } from './use-app-store';
import { useInspectorStore } from './use-inspector-store';
import { useLabStore } from './use-lab-store';

export function openPlanLink(
  project: string,
  node: PlanNode,
  to: LinkAction,
  version: number | null
): void {
  const app = useAppStore.getState();
  if (to === 'inspector' && node.asset && version !== null) {
    useInspectorStore.getState().focusVersion(project, node.asset, version);
    app.setView('inspector');
  } else if (to === 'lab') {
    if (node.direction) useLabStore.getState().select(node.direction);
    app.setView('lab');
  } else if (to === 'inbox') {
    app.setView('inbox');
  } else {
    app.setView('board');
  }
}
