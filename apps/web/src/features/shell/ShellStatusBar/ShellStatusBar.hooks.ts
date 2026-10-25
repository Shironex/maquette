import { useNow } from '@/lib/useNow';
import { mcpStatusItem } from '@/state/agents';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { useInboxStore } from '@/state/use-inbox-store';
import { useLabStore } from '@/state/use-lab-store';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { shellStatusItems } from './statusItems';

/** The status bar items, from the tools, the Lab, the inbox, the sends and the last run. */
export function useShellStatusBar() {
  const workspace = useAppStore(s => s.workspace);
  const { reports, checking } = useAppStore(s => s.tools);
  const view = useAppStore(s => s.view);
  const labJobs = useLabStore(s => s.view?.jobs) ?? [];
  const inbox = useInboxStore(s => s.snapshot);
  const pipeline = usePipelineStore(s => s.view);
  const now = useNow(15_000);
  const mcp = mcpStatusItem(useAgentsStore(s => s.status));
  const sends = useProvidersStore(s => s.jobs);
  return shellStatusItems({
    view,
    workspace,
    reports,
    checking,
    labJobs,
    inbox,
    latest: pipeline?.runs[0],
    now,
    mcp,
    sends,
  });
}
