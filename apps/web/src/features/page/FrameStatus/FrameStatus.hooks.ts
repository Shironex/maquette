import { useProjectsStore } from '@/state/use-projects-store';
import { useAppStore } from '@/state/use-app-store';
import { usePageStore } from '@/state/use-page-store';
import type { LogLine } from '@maquette/ui';

/** The server's log as console lines, the command first. */
export function logLines(log: readonly string[]): LogLine[] {
  return log.map((text, i) => ({
    t: String(i + 1).padStart(3, ' '),
    text,
    level: /error|failed|cannot|EADDRINUSE/i.test(text) ? ('error' as const) : undefined,
    dim: text.startsWith('$') || text.startsWith('>'),
  }));
}

/** What the states can do: start, switch here, pick a concept, open Settings. */
export function useFrameStatus() {
  const start = usePageStore(s => s.start);
  const pickConcept = usePageStore(s => s.pickConcept);
  const save = usePageStore(s => s.save);
  const setView = useAppStore(s => s.setView);
  const projects = useProjectsStore(s => s.list);
  return {
    start: () => void start(),
    pickConcept: () => void pickConcept(),
    showSite: () => void save({ source: 'site' }),
    openSettings: () => setView('settings'),
    projectName: (id: string) => projects.find(p => p.id === id)?.name ?? id,
  };
}
