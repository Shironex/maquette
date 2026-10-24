import { ProjectForm } from '@/features/project/ProjectForm';
import type { OnboardingApi } from '../useOnboarding';

export function ProjectStep({ api }: { api: OnboardingApi }) {
  return (
    <ProjectForm
      value={api.draft}
      onChange={api.update}
      actions={api.projectActions}
      locked={api.created}
    />
  );
}
