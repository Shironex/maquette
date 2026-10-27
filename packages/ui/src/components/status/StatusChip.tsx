import { Icon, type IconName } from '../../icons/Icon';
import { cn } from '../../lib/cn';

export type Status = 'done' | 'cached' | 'running' | 'failed' | 'review' | 'queued';

const CHIP: Record<Status, { icon: IconName; label: string; spin?: boolean; tone: string }> = {
  done: { icon: 'check', label: 'Done', tone: 'bg-sage-soft text-sage' },
  cached: { icon: 'stack', label: 'Cached', tone: 'bg-fg-dim/10 text-fg-dim' },
  running: { icon: 'circle-notch', label: 'Running', spin: true, tone: 'bg-amber-soft text-amber' },
  failed: { icon: 'x-circle', label: 'Failed', tone: 'bg-danger-soft text-danger' },
  review: { icon: 'eye', label: 'Needs review', tone: 'bg-clay-soft text-clay-hover' },
  queued: {
    icon: 'clock',
    label: 'Queued',
    tone: 'bg-transparent text-fg-mute shadow-[inset_0_0_0_1px_var(--line-strong)]',
  },
};

export interface StatusChipProps {
  status: Status;
  label?: string;
  /** Overrides the status icon; `false` hides it (state still reads from the word). */
  icon?: IconName | false;
  className?: string;
}

/**
 * State is never colour alone: every chip carries a word and, by default, an
 * icon. A word longer than the space ends in an ellipsis.
 */
export function StatusChip({ status, label, icon, className }: StatusChipProps) {
  const c = CHIP[status];
  return (
    <span
      className={cn(
        'inline-flex h-5 max-w-full items-center gap-[5px] rounded-pill pr-2 pl-1.5 text-[11.5px] leading-none font-medium whitespace-nowrap',
        icon === false && 'pl-2',
        c.tone,
        className
      )}
    >
      {icon === false ? null : (
        <Icon
          name={icon ?? c.icon}
          size={12}
          spin={icon === undefined && c.spin}
          className="flex-none"
        />
      )}
      <span className="min-w-0 truncate">{label ?? c.label}</span>
    </span>
  );
}
