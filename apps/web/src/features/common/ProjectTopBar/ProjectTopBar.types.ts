import type { ReactNode } from 'react';

export interface ProjectTopBarProps {
  /** A view's own action, shown before Run pipeline. */
  extra?: ReactNode;
  /** Run pipeline is this view's bright button; on the other views it stays quiet. */
  primaryRun?: boolean;
}
