import type { Approval } from '@/ipc/client';
import { useNow } from '@/lib/useNow';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';

/** The project an approval is for, the clock for its age, and the decision. */
export function useApprovalCard(approval: Approval) {
  const decide = useAgentsStore(s => s.decide);
  const deciding = useAgentsStore(s => s.deciding === approval.id);
  const projects = useAppStore(s => s.workspace?.projects);
  const site = projects?.find(p => p.id === approval.project);
  const now = useNow(15_000);
  return { decide, deciding, site, now };
}
