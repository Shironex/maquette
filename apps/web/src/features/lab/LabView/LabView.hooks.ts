import { useEffect } from 'react';
import { useAppStore } from '@/state/use-app-store';
import { useCodexUsageStore } from '@/state/use-codex-usage-store';
import { useLabStore } from '@/state/use-lab-store';
import { useWait } from '@/lib/waiting';

/** The open project and the lab view loaded last; the Codex usage is read every minute while it is open. */
export function useLabView() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const view = useLabStore(s => s.view);
  const poll = useCodexUsageStore(s => s.poll);
  useEffect(() => poll(), [poll]);
  const failed = useLabStore(s => s.loadFailed);
  const load = useLabStore(s => s.load);
  const reading = Boolean(project) && view?.project !== project?.id && !failed;
  const estimate = useWait(project ? `lab:${project.id}` : null, reading, failed);
  return { project, view, failed, estimate, retry: () => project && void load(project.id) };
}
