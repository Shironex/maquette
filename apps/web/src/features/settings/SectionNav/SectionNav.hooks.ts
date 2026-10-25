import { useAppStore } from '@/state/use-app-store';
import { summarizeTools } from '@/state/toolchain';
import { projectThumb } from '@/features/common/projectArt';

/** The toolchain count and the open project the nav lists. */
export function useSectionNav() {
  const reports = useAppStore(s => s.tools.reports);
  const project = useAppStore(s => s.workspace?.project);
  return {
    checked: reports.length > 0,
    summary: summarizeTools(reports),
    project,
    thumb: project ? projectThumb(project) : undefined,
  };
}
