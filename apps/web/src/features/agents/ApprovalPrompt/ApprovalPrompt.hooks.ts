import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';

/** The approvals to float over the current view; Settings > Agents lists them in place. */
export function useFloatingApprovals() {
  const approvals = useAgentsStore(s => s.status?.approvals);
  const view = useAppStore(s => s.view);
  return view === 'settings' ? undefined : approvals;
}
