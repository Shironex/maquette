import { ProjectForm } from '@/features/project/ProjectForm';
import type { ProjectStepProps } from './ProjectStep.types';

export default function ProjectStep({ api }: ProjectStepProps) {
  return (
    <ProjectForm
      value={api.draft}
      onChange={api.update}
      actions={api.projectActions}
      locked={api.created}
    />
  );
}
