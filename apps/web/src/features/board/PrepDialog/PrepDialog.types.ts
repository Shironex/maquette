import type { Asset } from '@/ipc/client';

export interface PrepDialogProps {
  project: string;
  asset: Asset;
  onClose: () => void;
}
