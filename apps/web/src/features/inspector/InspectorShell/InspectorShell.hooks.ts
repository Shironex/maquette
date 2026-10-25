import { stageViews } from '@/state/pipeline';
import { useAppStore } from '@/state/use-app-store';
import { usePipelineStore } from '@/state/use-pipeline-store';

/** The project's pipeline stages for the stepper, or null without a project. */
export function useInspectorShell() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const latest = usePipelineStore(s => s.view?.runs[0]);
  const stages = project ? stageViews(project.assets ?? [], latest) : null;
  return { stages };
}
