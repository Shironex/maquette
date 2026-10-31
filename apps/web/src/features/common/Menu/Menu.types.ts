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
  /** A number shown beside the label, in the item's accessible name too. */
  count?: number;
  /** `danger` marks an item that removes something, in the danger colour. */
  tone?: 'danger';
  /** Leaves the menu open, for an item that leads to more choices. */
  keepOpen?: boolean;
  /**
   * The items sit together with the others of the same section; a line
   * parts one section from the next.
   */
  section?: string;
  /**
   * A submenu that opens beside the item, on hover, a click or ArrowRight;
   * ArrowLeft and Escape close it again.
   */
  submenu?: readonly MenuItem[];
  /** Not called for an item with a submenu. */
  onSelect?: () => void;
}

export interface MenuProps {
  label: string;
  items: readonly MenuItem[];
  onClose: () => void;
  header?: ReactNode;
  className?: string;
}
