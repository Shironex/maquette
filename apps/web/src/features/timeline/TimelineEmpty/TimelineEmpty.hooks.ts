import { useAppStore } from '@/state/use-app-store';

/** A project without camera stops: where to make them. */
export function useTimelineEmpty() {
  const name = useAppStore(s => s.workspace?.project?.name ?? 'This project');
  const setView = useAppStore(s => s.setView);
  return { name, openInspector: () => setView('inspector') };
}
