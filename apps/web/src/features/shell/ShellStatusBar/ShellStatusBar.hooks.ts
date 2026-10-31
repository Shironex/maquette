import { useEffect } from 'react';
import { useNow } from '@/lib/useNow';
import { mcpStatusItem } from '@/state/agents';
import { runStatusLine } from '@/state/pipeline';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { useInboxStore } from '@/state/use-inbox-store';
import { useLabStore } from '@/state/use-lab-store';
import { useAnimationsStore } from '@/state/use-animations-store';
import { useCodexUsageStore } from '@/state/use-codex-usage-store';
import { useLibraryStore } from '@/state/use-library-store';
import { usePageStore } from '@/state/use-page-store';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { renderStatusLine } from '@/state/renders';
import { useRenderStore } from '@/state/use-render-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { shellStatusItems, type ShellStatus } from './statusItems';

/**
 * The status bar items, from the tools, the Lab, the Codex usage, the inbox,
 * the sends and the last run; `siteControl` draws the site dev server's item
 * and `codexControl` the Codex usage.
 */
export function useShellStatusBar(
  siteControl?: ShellStatus['siteControl'],
  codexControl?: ShellStatus['codexControl']
) {
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
  const codexUsage = useCodexUsageStore(s => s.usage);
  const rendering = renderStatusLine(useRenderStore(s => s.live));
  const projectId = workspace?.project?.id;
  useEffect(() => {
    if (projectId) void useRenderStore.getState().loadRecent();
  }, [projectId]);
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
    rendering,
    siteControl,
    libraryModels,
    animationClips: animations?.items.length ?? null,
    animationsDir: animations?.dir ?? null,
    codexUsage,
    codexControl,
  });
}

/** The latest run's line (its step, or where it failed), for a polite live region. */
export function useRunAnnouncement(): string {
  const latest = usePipelineStore(s => s.view?.runs[0]);
  return latest ? runStatusLine(latest) : '';
}
