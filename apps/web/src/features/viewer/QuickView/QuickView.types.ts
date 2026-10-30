import type { QuickViewTarget } from '@/state/quick-view';

export interface QuickViewDialogProps {
  target: QuickViewTarget;
  onClose: () => void;
}
