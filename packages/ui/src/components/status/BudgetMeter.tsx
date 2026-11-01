import { cn } from '../../lib/cn';
import { budgetScale } from './BudgetBar';

export interface BudgetMeterProps {
  label: string;
  /** What the label shrinks to in a narrow top bar; the meter's name stays whole. */
  shortLabel?: string;
  /** Null while the number is not known: the meter says so instead of showing a zero. */
  value: number | null;
  target: number;
  unit: string;
  /** What it says while `value` is null. */
  pending?: string;
  format?: (n: number) => string;
  className?: string;
}

/** The compact budget read for the top bar: one line of numbers over a 4 px bar. */
export function BudgetMeter({
  label,
  shortLabel,
  value,
  target,
  unit,
  format = n => n.toFixed(1),
  pending = 'Reading...',
  className,
}: BudgetMeterProps) {
  const known = value !== null;
  const { over, fill } = budgetScale(value ?? 0, target);
  return (
    <div className={cn('flex min-w-42 flex-col gap-[5px] @max-[1000px]:min-w-0', className)}>
      <div className="flex items-baseline justify-between gap-3 text-[12px] leading-[14px] text-fg-mute">
        {shortLabel ? (
          <>
            <span className="@max-[1000px]:hidden">{label}</span>
            <span className="hidden @max-[1000px]:inline">{shortLabel}</span>
          </>
        ) : (
          <span>{label}</span>
        )}
        {known ? (
          <span className={cn('font-mono text-[12px]', over ? 'text-danger' : 'text-fg')}>
            {format(value)} <span className="text-fg-mute">{`/ ${format(target)} ${unit}`}</span>
          </span>
        ) : (
          <span className="text-[12px] text-fg-mute">{pending}</span>
        )}
      </div>
      <div
        role="meter"
        aria-label={
          known ? `${label}, ${format(value)} of ${format(target)} ${unit}` : `${label}, ${pending}`
        }
        aria-valuenow={value ?? 0}
        aria-valuetext={known ? undefined : pending}
        aria-valuemin={0}
        aria-valuemax={target}
        className="relative h-1 rounded-pill bg-hover"
      >
        {known && value > 0 ? (
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
