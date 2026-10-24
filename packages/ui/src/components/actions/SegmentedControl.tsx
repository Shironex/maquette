import { Icon, type IconName } from '../../icons/Icon';
import { cn } from '../../lib/cn';
import { useControllable } from '../../lib/useControllable';

export interface SegmentedOption<T extends string> {
  value: T;
  label?: string;
  icon?: IconName;
}

export interface SegmentedControlProps<T extends string> {
  options: readonly SegmentedOption<T>[];
  value?: T;
  defaultValue?: T;
  onChange?: (value: T) => void;
  /** Accessible name of the group. */
  label: string;
  /** `clay` marks a setting choice; `neutral` a view switch. */
  tone?: 'clay' | 'neutral';
  block?: boolean;
  className?: string;
}

export function SegmentedControl<T extends string>({
  options,
  value,
  defaultValue,
  onChange,
  label,
  tone = 'neutral',
  block,
  className,
}: SegmentedControlProps<T>) {
  const first = options[0]?.value;
  const [current, set] = useControllable<T | undefined>(value, defaultValue ?? first, next => {
    if (next !== undefined) onChange?.(next);
  });
  return (
    <div
      role="group"
      aria-label={label}
      className={cn(
        'inline-flex gap-0.5 rounded-[8px] border border-line bg-app p-0.5',
        block && 'flex w-full',
        className
      )}
    >
      {options.map(o => {
        const on = o.value === current;
        return (
          <button
            key={o.value}
            type="button"
            aria-pressed={on}
            onClick={() => set(o.value)}
            className={cn(
              'inline-flex h-[26px] cursor-pointer items-center justify-center gap-1.5 whitespace-nowrap rounded-control px-2.5 text-[12px] font-medium text-fg-dim transition-colors duration-120 hover:text-fg',
              block && 'flex-1',
              on &&
                (tone === 'clay'
                  ? 'bg-clay-soft text-fg shadow-[inset_0_0_0_1px_var(--clay-line)]'
                  : 'bg-hover text-fg shadow-[inset_0_0_0_1px_var(--line-strong)]')
            )}
          >
            {o.icon ? <Icon name={o.icon} size={14} /> : null}
            {o.label}
          </button>
        );
      })}
    </div>
  );
}
