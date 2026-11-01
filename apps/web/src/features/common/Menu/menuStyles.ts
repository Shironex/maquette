/** One item of a menu or submenu. */
export const MENU_ROW =
  'flex w-full cursor-pointer items-center gap-2 rounded-control px-2 py-1.5 text-left text-[13px] hover:bg-hover focus-visible:bg-hover focus-visible:outline-none active:bg-line disabled:cursor-default disabled:text-fg-mute disabled:hover:bg-transparent aria-disabled:cursor-default aria-disabled:text-fg-mute aria-disabled:hover:bg-transparent aria-expanded:bg-hover';

/** A row that removes something: the danger colour, and its soft fill on hover. */
export const MENU_ROW_DANGER = 'text-danger hover:bg-danger-soft focus-visible:bg-danger-soft';

/** The line between two sections. */
export const MENU_SEPARATOR = 'mx-1 my-1 h-px bg-line';

/** The heading over a section. */
export const MENU_SECTION_LABEL =
  'px-2 pt-1 pb-0.5 text-caption tracking-[0.06em] text-fg-mute uppercase';

/** The second line of an item: why it is disabled, or what it does. */
export const MENU_ROW_DETAIL = 'block text-meta whitespace-normal text-fg-mute';

/** The panel a menu or submenu sits on. */
export const MENU_PANEL = 'rounded-card border border-line-strong bg-panel p-1 shadow-window';
