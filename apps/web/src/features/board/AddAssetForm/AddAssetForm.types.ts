import type { ProjectManifest } from '@/ipc/client';

export interface AddAssetFormProps {
  project: string;
  onDone: (project: ProjectManifest) => void;
  onCancel: () => void;
}
