import type { ReactNode } from 'react';

export interface PanelProps {
  title?: string;
  /** A short line next to or under the title. */
  help?: string;
  helpInline?: boolean;
  actions?: ReactNode;
  id?: string;
  className?: string;
  children: ReactNode;
}
