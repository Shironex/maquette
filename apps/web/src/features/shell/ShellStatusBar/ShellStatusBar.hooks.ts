import { useNow } from '@/lib/useNow';
import { mcpStatusItem } from '@/state/agents';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { useInboxStore } from '@/state/use-inbox-store';
import { useLabStore } from '@/state/use-lab-store';
import { useAnimationsStore } from '@/state/use-animations-store';
import { useLibraryStore } from '@/state/use-library-store';
import { usePageStore } from '@/state/use-page-store';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { shellStatusItems, type ShellStatus } from './statusItems';

/**
 * The status bar items, from the tools, the Lab, the inbox, the sends and
 * the last run; `siteControl` draws the site dev server's item.
 */
export function useShellStatusBar(siteControl?: ShellStatus['siteControl']) {
  const workspace = useAppStore(s => s.workspace);
  const { reports, checking } = useAppStore(s => s.tools);
  const view = useAppStore(s => s.view);
  const labJobs = useLabStore(s => s.view?.jobs) ?? [];
  const inbox = useInboxStore(s => s.snapshot);
  const pipeline = usePipelineStore(s => s.view);
  const now = useNow(15_000);
  const mcp = mcpStatusItem(useAgentsStore(s => s.status));
  const sends = useProvidersStore(s => s.jobs);
  const server = usePageStore(s => s.server);
  const libraryModels = useLibraryStore(s => s.view?.items.length ?? null);
  const animations = useAnimationsStore(s => s.view);
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
    server,
    siteControl,
    libraryModels,
    animationClips: animations?.items.length ?? null,
    animationsDir: animations?.dir ?? null,
  });
}
