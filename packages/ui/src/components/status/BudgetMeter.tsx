import { cn } from '../../lib/cn';
import { budgetScale } from './BudgetBar';

export interface BudgetMeterProps {
  label: string;
  value: number;
  target: number;
  unit: string;
  format?: (n: number) => string;
  className?: string;
}

/** The compact budget read for the top bar: one line of numbers over a 4 px bar. */
export function BudgetMeter({
  label,
  value,
  target,
  unit,
  format = n => n.toFixed(1),
  className,
}: BudgetMeterProps) {
  const { over, fill } = budgetScale(value, target);
  return (
    <div className={cn('flex min-w-42 flex-col gap-[5px]', className)}>
      <div className="flex items-baseline justify-between gap-3 text-[12px] leading-[14px] text-fg-mute">
        <span>{label}</span>
        <span className={cn('font-mono text-[12px]', over ? 'text-danger' : 'text-fg')}>
          {format(value)} <span className="text-fg-mute">{`/ ${format(target)} ${unit}`}</span>
        </span>
      </div>
      <div
        role="meter"
        aria-label={`${label}, ${format(value)} of ${format(target)} ${unit}`}
        aria-valuenow={value}
        aria-valuemin={0}
        aria-valuemax={target}
        className="relative h-1 rounded-pill bg-hover"
      >
        {value > 0 ? (
          <i
            className={cn(
              'absolute inset-y-0 left-0 block rounded-pill',
              over ? 'bg-danger' : 'bg-sage'
            )}
            style={{ width: `${Math.min(100, fill)}%` }}
          />
        ) : null}
      </div>
    </div>
  );
}
