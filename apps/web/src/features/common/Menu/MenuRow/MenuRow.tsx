import { Icon, Tooltip, cn } from '@maquette/ui';
import type { MenuItem } from '../Menu.types';
import { startsSection } from '../menuKeys';
import { MENU_PANEL, MENU_ROW, MENU_SEPARATOR } from '../menuStyles';
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
  const sub = useSubmenu(Boolean(item.submenu) && !blocked);
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
        className={MENU_ROW}
      >
        {item.icon ? <Icon name={item.icon} size={14} className="text-fg-dim" /> : null}
        <span className="min-w-0 flex-1 truncate">{item.label}</span>
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
        // The padding bridges the gap, so the pointer can cross to the submenu.
        <div
          className={cn('absolute -top-1 z-30', sub.flip ? 'right-full pr-1' : 'left-full pl-1')}
        >
          <div
            ref={sub.menu}
            role="menu"
            aria-label={item.label}
            onKeyDown={sub.onMenuKey}
            style={{ width: SUBMENU_WIDTH }}
            className={MENU_PANEL}
          >
            {rows}
          </div>
        </div>
      ) : null}
    </div>
  );
}
