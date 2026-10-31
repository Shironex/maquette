import { Icon, Tooltip, cn } from '@maquette/ui';
import type { MenuItem } from '../Menu.types';
import { startsSection } from '../menuKeys';
import { EDGE } from '../menuPlacement';
import { MENU_PANEL, MENU_ROW, MENU_ROW_DANGER, MENU_SEPARATOR } from '../menuStyles';
import { SUBMENU_WIDTH, useSubmenu } from './MenuRow.hooks';

export interface MenuRowProps {
  item: MenuItem;
  /** Closes the whole menu, after a pick. */
  onClose: () => void;
}

/** One item of a menu: a button, with its submenu beside it when it has one. */
export default function MenuRow({ item, onClose }: MenuRowProps) {
  const explained = Boolean(item.disabled && item.hint);
  const blocked = Boolean(item.disabled);
  const sub = useSubmenu(Boolean(item.submenu) && !blocked, item.submenu?.length ?? 0);
  const button = (
    <Tooltip label={item.hint} side="right" disabled={!item.hint || sub.open}>
      <button
        ref={sub.button}
        type="button"
        role="menuitem"
        disabled={item.disabled && !explained}
        aria-disabled={explained || undefined}
        aria-haspopup={item.submenu ? 'menu' : undefined}
        aria-expanded={item.submenu ? sub.open : undefined}
        onKeyDown={item.submenu ? sub.onButtonKey : undefined}
        onClick={e => {
          if (explained) return;
          if (item.submenu) return sub.onClick(e);
          item.onSelect?.();
          if (!item.keepOpen) onClose();
        }}
        className={cn(MENU_ROW, item.tone === 'danger' && MENU_ROW_DANGER)}
      >
        {item.icon ? (
          <Icon
            name={item.icon}
            size={14}
            className={item.tone === 'danger' ? 'text-danger' : 'text-fg-dim'}
          />
        ) : null}
        <span className="min-w-0 flex-1 truncate">{item.label}</span>
        {item.count !== undefined ? (
          <span className="rounded-pill bg-raised px-1.5 font-mono text-[11px] leading-[18px] text-fg-dim">
            {item.count}
          </span>
        ) : null}
        {item.submenu ? <Icon name="caret-right" size={12} className="text-fg-mute" /> : null}
      </button>
    </Tooltip>
  );
  if (!item.submenu) return button;
  const children = item.submenu;
  const rows = children.map((child, i) => (
    <div key={child.id} className="contents">
      {startsSection(children, i) ? <div role="separator" className={MENU_SEPARATOR} /> : null}
      <MenuRow item={child} onClose={onClose} />
    </div>
  ));
  return (
    <div
      className="relative"
      onPointerEnter={sub.onPointerEnter}
      onPointerLeave={sub.onPointerLeave}
    >
      {button}
      {sub.open ? (
        // Fixed, so a scrolling column cannot clip it; the padding bridges the
        // gap, so the pointer can cross to it.
        <div
          className="fixed z-30"
          style={{
            top: sub.place.top,
            left: sub.place.left,
            paddingLeft: sub.place.padLeft,
            paddingRight: sub.place.padRight,
          }}
        >
          <div
            ref={sub.menu}
            role="menu"
            aria-label={item.label}
            onKeyDown={sub.onMenuKey}
            style={{ width: SUBMENU_WIDTH, maxHeight: window.innerHeight - sub.place.top - EDGE }}
            className={cn(MENU_PANEL, 'overflow-y-auto overscroll-contain')}
          >
            {rows}
          </div>
        </div>
      ) : null}
    </div>
  );
}
