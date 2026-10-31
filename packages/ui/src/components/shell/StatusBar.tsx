import type { ReactNode } from 'react';
import { Icon, type IconName } from '../../icons/Icon';
import { cn } from '../../lib/cn';

export type StatusDot = 'ok' | 'running' | 'offline' | 'idle';

export interface StatusBarItem {
  label: string;
  icon?: IconName;
  dot?: StatusDot;
  /** Pushes this item and the ones after it to the right edge. */
  right?: boolean;
  /**
   * Replaces the dot and label with a control of its own, for an item that
   * acts (a menu of what to do with it). `label` stays its key.
   */
  node?: ReactNode;
}

const DOT: Record<StatusDot, string> = {
  ok: 'bg-sage',
  running: 'bg-amber shadow-[0_0_0_3px_var(--amber-soft)]',
  offline: 'bg-danger',
  idle: 'bg-line-strong',
};

/** The small coloured dot an item leads with. */
export function StatusDotMark({ dot }: { dot: StatusDot }) {
  return <span aria-hidden="true" className={cn('size-[7px] flex-none rounded-full', DOT[dot])} />;
}

/**
 * One line of items, most important first. An item that does not fit wraps
 * to a second line the bar clips, so a narrow window loses the last items
 * whole instead of cutting one in half.
 */
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
      className={cn('flex h-7 items-center gap-1.5 whitespace-nowrap', it.right && 'ml-auto')}
    >
      {it.node ?? (
        <>
          {it.dot ? (
            <StatusDotMark dot={it.dot} />
          ) : it.icon ? (
            <Icon name={it.icon} size={12} />
          ) : null}
          {it.label}
        </>
      )}
    </span>
  ));
  return (
    <footer
      className={cn(
        'flex h-7 flex-none flex-wrap content-start gap-x-[18px] overflow-hidden border-t border-line bg-panel px-4 text-[11.5px] text-fg-mute',
        className
      )}
    >
      {statusItems}
    </footer>
  );
}
