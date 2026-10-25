import type { ReactNode } from 'react';
import type { IconName } from '@maquette/ui';

export interface MenuItem {
  id: string;
  label: string;
  icon?: IconName;
  disabled?: boolean;
  /**
   * A tooltip beside the item. On a disabled item it says why, and the
   * item stays focusable so the keyboard reaches it too.
   */
  hint?: string;
  /** Leaves the menu open, for an item that leads to more choices. */
  keepOpen?: boolean;
  onSelect: () => void;
}

export interface MenuProps {
  label: string;
  items: readonly MenuItem[];
  onClose: () => void;
  header?: ReactNode;
  className?: string;
}
