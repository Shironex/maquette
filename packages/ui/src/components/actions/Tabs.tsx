import { cn } from '../../lib/cn';
import { useControllable } from '../../lib/useControllable';

export interface TabItem {
  id: string;
  label: string;
  count?: number;
}

export interface TabsProps {
  tabs: readonly TabItem[];
  value?: string;
  defaultValue?: string;
  onChange?: (id: string) => void;
  className?: string;
}

export function Tabs({ tabs, value, defaultValue, onChange, className }: TabsProps) {
  const [current, set] = useControllable(value, defaultValue ?? tabs[0]?.id ?? '', onChange);
  const tabButtons = tabs.map(t => {
    const on = t.id === current;
    return (
      <button
        key={t.id}
        type="button"
        role="tab"
        aria-selected={on}
        onClick={() => set(t.id)}
        className={cn(
          'relative inline-flex h-9 cursor-pointer items-center gap-1.5 font-medium text-fg-dim transition-colors duration-120 hover:text-fg active:text-fg-mute',
          on &&
            'text-fg after:absolute after:inset-x-0 after:-bottom-px after:h-0.5 after:rounded-[2px] after:bg-clay'
        )}
      >
        {t.label}
        {t.count !== undefined ? (
          <span
            className={cn(
              'rounded-pill px-1.5 text-[11px] leading-4 tabular-nums',
              on ? 'bg-clay-soft text-clay-text' : 'bg-raised text-fg-dim'
            )}
          >
            {t.count}
          </span>
        ) : null}
      </button>
    );
  });
  return (
    <div role="tablist" className={cn('flex gap-4 border-b border-line', className)}>
      {tabButtons}
    </div>
  );
}
