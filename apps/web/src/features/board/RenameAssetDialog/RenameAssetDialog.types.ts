import type { Asset, ProjectManifest } from '@/ipc/client';

export interface RenameAssetDialogProps {
  project: string;
  asset: Asset;
  onDone: (project: ProjectManifest) => void;
  onClose: () => void;
}
