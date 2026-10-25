import { Icon, type IconName } from '../../icons/Icon';
import { cn } from '../../lib/cn';

export type StatusDot = 'ok' | 'running' | 'offline' | 'idle';

export interface StatusBarItem {
  label: string;
  icon?: IconName;
  dot?: StatusDot;
  /** Pushes this item and the ones after it to the right edge. */
  right?: boolean;
}

const DOT: Record<StatusDot, string> = {
  ok: 'bg-sage',
  running: 'bg-amber shadow-[0_0_0_3px_var(--amber-soft)]',
  offline: 'bg-danger',
  idle: 'bg-line-strong',
};

export function StatusBar({
  items,
  className,
}: {
  items: readonly StatusBarItem[];
  className?: string;
}) {
  const statusItems = items.map((it, i) => (
    <span
      key={`${i}-${it.label}`}
      className={cn('flex items-center gap-1.5 whitespace-nowrap', it.right && 'ml-auto')}
    >
      {it.dot ? (
        <span aria-hidden="true" className={cn('size-[7px] flex-none rounded-full', DOT[it.dot])} />
      ) : it.icon ? (
        <Icon name={it.icon} size={12} />
      ) : null}
      {it.label}
    </span>
  ));
  return (
    <footer
      className={cn(
        'flex h-7 flex-none items-center gap-[18px] border-t border-line bg-panel px-4 text-[11.5px] text-fg-mute',
        className
      )}
    >
      {statusItems}
    </footer>
  );
}
