import { Icon, StatusDotMark, Tooltip, type StatusDot } from '@maquette/ui';
import { Menu } from '@/features/common/Menu';
import { useSiteServerItem } from './SiteServerItem.hooks';

export interface SiteServerItemProps {
  label: string;
  dot: StatusDot;
}

/** The status bar's site dev server: its state, and a menu to open, show or stop it. */
export default function SiteServerItem({ label, dot }: SiteServerItemProps) {
  const { open, toggle, close, items, detail, command } = useSiteServerItem();
  const header = (
    <div className="flex max-w-[320px] flex-col gap-0.5 px-2 pt-1 pb-1.5">
      <span className="text-[12px] text-fg-dim">{detail}</span>
      {command ? (
        <span className="truncate font-mono text-[11.5px] text-fg-mute">{command}</span>
      ) : null}
    </div>
  );
  return (
    <span className="relative flex">
      <Tooltip label="Site dev server" detail={detail} disabled={open}>
        <button
          type="button"
          aria-haspopup="menu"
          aria-expanded={open}
          onClick={toggle}
          className="-mx-1.5 flex h-6 cursor-pointer items-center gap-1.5 rounded-control px-1.5 hover:bg-hover hover:text-fg aria-expanded:bg-hover aria-expanded:text-fg"
        >
          <StatusDotMark dot={dot} />
          {label}
          <Icon name="caret-up-down" size={11} />
        </button>
      </Tooltip>
      {open ? (
        <Menu
          label="Site dev server"
          items={items}
          header={header}
          onClose={close}
          className="absolute bottom-full left-0 mb-1"
        />
      ) : null}
    </span>
  );
}
