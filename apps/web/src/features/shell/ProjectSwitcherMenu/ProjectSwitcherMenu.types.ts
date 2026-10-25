import type { RefObject } from 'react';

export interface ProjectSwitcherMenuProps {
  anchorRef: RefObject<HTMLButtonElement | null>;
  collapsed: boolean;
  shortcut: string;
}
