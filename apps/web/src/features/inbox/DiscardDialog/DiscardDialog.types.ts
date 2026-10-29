import type { InboxItem } from '@/ipc/client';

export interface DiscardDialogProps {
  item: InboxItem;
  onClose: () => void;
}
