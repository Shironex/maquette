import type { ReactNode } from 'react';

export interface BoardViewProps {
  /** The 3D queue panel, supplied by the shell so the Board stays its own feature. */
  queue: (onClose: () => void) => ReactNode;
}
