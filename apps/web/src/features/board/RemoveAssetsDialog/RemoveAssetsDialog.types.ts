import type { Asset, ProviderJob } from '@/ipc/client';

export interface RemoveAssetsDialogProps {
  project: string;
  /** The cards to remove: one from its menu, or the selection. */
  assets: readonly Asset[];
  /** The project's sends, to say which ones the remove stops. */
  jobs: readonly ProviderJob[];
  /** After a remove went through, whatever each asset answered. */
  onDone: () => void;
  onClose: () => void;
}
