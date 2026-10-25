import type { ReactNode } from 'react';
import type { IconName } from '@maquette/ui';

export interface SheetProps {
  /** Prefix for the heading id. */
  id: string;
  icon: IconName;
  title: string;
  meta?: ReactNode;
  onClose: () => void;
  /** Unsent changes: Escape, the backdrop and the close button ask first. */
  dirty?: boolean;
  width?: number;
  children: ReactNode;
}
