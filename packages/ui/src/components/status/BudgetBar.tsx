import { Icon } from '../../icons/Icon';
import { cn } from '../../lib/cn';

export interface BudgetBarProps {
  label: string;
  value: number;
  target: number;
  unit?: string;
  format?: (n: number) => string;
  showNote?: boolean;
  className?: string;
}

/** Scale so the target mark sits at 80 percent until the value passes it. */
export function budgetScale(value: number, target: number) {
  const over = value > target;
  const max = over ? value : Math.max(target * 1.25, value);
  return {
    over,
    fill: max > 0 ? (value / max) * 100 : 0,
    targetAt: max > 0 ? (target / max) * 100 : 0,
  };
}

export function BudgetBar({
  label,
  value,
  target,
  unit,
  format = n => String(n),
  showNote = true,
  className,
}: BudgetBarProps) {
  const { over, fill, targetAt } = budgetScale(value, target);
  const suffix = unit ? ` ${unit}` : '';
  return (
    <div className={cn('flex min-w-40 flex-col gap-1.5', className)}>
      <div className="flex items-baseline justify-between gap-3 text-meta">
        <span className="text-fg-dim">{label}</span>
        <span className="whitespace-nowrap text-fg-mute tabular-nums">
          <b className={cn('font-semibold', over ? 'text-danger' : 'text-fg')}>{format(value)}</b>
          {` / ${format(target)}${suffix}`}
        </span>
      </div>
      <div
        role="meter"
        aria-label={label}
        aria-valuenow={value}
        aria-valuemin={0}
        aria-valuemax={target}
        className="relative h-1.5 rounded-pill bg-hover"
      >
        <div
          className={cn('absolute inset-y-0 left-0 rounded-pill', over ? 'bg-danger' : 'bg-sage')}
          style={{ width: `${fill}%` }}
        />
        <div
          title="Target"
          className="absolute -inset-y-[3px] w-0.5 rounded-[1px] bg-fg-dim"
          style={{ left: `calc(${targetAt}% - 1px)` }}
        />
      </div>
      {over && showNote ? (
        <span className="inline-flex items-center gap-1 text-meta text-danger">
          <Icon name="warning" size={12} />
          {`${format(Math.round((value - target) * 100) / 100)}${suffix} over target`}
        </span>
      ) : null}
    </div>
  );
}
