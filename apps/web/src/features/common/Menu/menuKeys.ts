/** The items of `menu` itself, not of a submenu inside it, that take focus. */
export function menuItems(menu: HTMLElement): HTMLElement[] {
  return [...menu.querySelectorAll<HTMLElement>('[role="menuitem"]')].filter(
    item => item.closest('[role="menu"]') === menu && !item.hasAttribute('disabled')
  );
}

const STEPS: Record<string, 'next' | 'prev' | 'first' | 'last'> = {
  ArrowDown: 'next',
  ArrowUp: 'prev',
  Home: 'first',
  End: 'last',
};

/**
 * Moves focus within `menu` for ArrowDown, ArrowUp (both wrap), Home and
 * End. True when the key was one of them.
 */
export function moveFocus(menu: HTMLElement, key: string): boolean {
  const to = STEPS[key];
  if (!to) return false;
  const list = menuItems(menu);
  if (list.length === 0) return true;
  const at = list.indexOf(document.activeElement as HTMLElement);
  const last = list.length - 1;
  const index =
    to === 'first'
      ? 0
      : to === 'last'
        ? last
        : to === 'next'
          ? at < 0 || at === last
            ? 0
            : at + 1
          : at <= 0
            ? last
            : at - 1;
  list[index]?.focus();
  return true;
}

/** The items cut into runs of one section, in order. */
export function sectionRuns<T extends { section?: string }>(items: readonly T[]): T[][] {
  const runs: T[][] = [];
  items.forEach((item, i) => {
    if (i === 0 || startsSection(items, i)) runs.push([item]);
    else runs[runs.length - 1]?.push(item);
  });
  return runs;
}

/** True when item `i` starts a new section, so a line goes above it. */
export function startsSection(items: readonly { section?: string }[], i: number): boolean {
  return i > 0 && items[i]?.section !== items[i - 1]?.section;
}
