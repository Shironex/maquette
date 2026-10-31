import { Icon, type IconName } from '../../icons/Icon';
import { cn } from '../../lib/cn';
import { useControllable } from '../../lib/useControllable';
import { Tooltip } from '../feedback/Tooltip';

export interface SegmentedOption<T extends string> {
  value: T;
  label?: string;
  icon?: IconName;
  /** The key that picks it, shown in a tooltip. */
  shortcut?: string;
  /** A tooltip line, for an icon-only option or one that needs a word more. */
  hint?: string;
  /**
   * Shown but not chosen. It stays focusable, so the keyboard reaches its
   * hint, which says why.
   */
  disabled?: boolean;
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
  /** The whole control is shown but locked: dimmed, and every option says so. */
  disabled?: boolean;
  className?: string;
  /**
   * Classes for each option's label, such as a container query that hides it
   * (`@max-[900px]:sr-only`) so the options shrink to their icons in a narrow bar.
   */
  labelClassName?: string;
}

export function SegmentedControl<T extends string>({
  options,
  value,
  defaultValue,
  onChange,
  label,
  tone = 'neutral',
  block,
  disabled: locked,
  className,
  labelClassName,
}: SegmentedControlProps<T>) {
  const first = options[0]?.value;
  const [current, set] = useControllable<T | undefined>(value, defaultValue ?? first, next => {
    if (next !== undefined) onChange?.(next);
  });
  const buttons = options.map(o => {
    const on = o.value === current;
    const off = locked || o.disabled;
    const tip = o.hint ?? o.label;
    return (
      <Tooltip key={o.value} label={tip} shortcut={o.shortcut} disabled={!o.shortcut && !o.hint}>
        <button
          type="button"
          aria-pressed={on}
          aria-label={o.label ? undefined : o.hint}
          aria-disabled={off || undefined}
          onClick={() => {
            if (!off) set(o.value);
          }}
          className={cn(
            'inline-flex h-[26px] cursor-pointer items-center justify-center gap-1.5 whitespace-nowrap rounded-control px-2.5 text-[12px] font-medium text-fg-dim transition-colors duration-120 hover:text-fg active:bg-raised',
            // Dimmed, and still 4.5:1 on the track: its name and hint must stay readable.
            o.disabled &&
              'cursor-not-allowed text-fg-mute opacity-80 hover:text-fg-mute active:bg-transparent',
            block && 'flex-1',
            on &&
              (tone === 'clay'
                ? 'bg-clay-soft text-fg shadow-[inset_0_0_0_1px_var(--clay-line)]'
                : 'bg-hover text-fg shadow-[inset_0_0_0_1px_var(--line-strong)]')
          )}
        >
          {o.icon ? <Icon name={o.icon} size={14} /> : null}
          {o.disabled && !o.icon ? <Icon name="lock-simple" size={12} /> : null}
          {labelClassName && o.label ? <span className={labelClassName}>{o.label}</span> : o.label}
        </button>
      </Tooltip>
    );
  });
  return (
    <div
      role="group"
      aria-label={label}
      className={cn(
        'inline-flex gap-0.5 rounded-[8px] border border-line bg-app p-0.5',
        block && 'flex w-full',
        locked && 'pointer-events-none opacity-60',
        className
      )}
    >
      {buttons}
    </div>
  );
}
