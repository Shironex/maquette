import { Icon } from '../../icons/Icon';
import { useControllable } from '../../lib/useControllable';

export interface NumberStepperProps {
  label?: string;
  value?: number;
  defaultValue?: number;
  min?: number;
  max?: number;
  step?: number;
  unit?: string;
  format?: (value: number) => string;
  onChange?: (value: number) => void;
}

export function NumberStepper({
  label,
  value,
  defaultValue,
  min,
  max,
  step = 1,
  unit,
  format = String,
  onChange,
}: NumberStepperProps) {
  const [current, set] = useControllable(value, defaultValue ?? min ?? 0, onChange);
  function nudge(delta: number) {
    let next = Math.round((current + delta) * 1000) / 1000;
    if (min !== undefined) next = Math.max(min, next);
    if (max !== undefined) next = Math.min(max, next);
    set(next);
  }
  const btn =
    'inline-flex h-full w-[30px] cursor-pointer items-center justify-center text-fg-dim hover:enabled:bg-hover hover:enabled:text-fg disabled:cursor-not-allowed disabled:opacity-40';
  return (
    <span
      role="group"
      aria-label={label}
      className="inline-flex h-8 items-center rounded-control border border-line-strong bg-raised"
    >
      <button
        type="button"
        aria-label="Decrease"
        disabled={min !== undefined && current <= min}
        onClick={() => nudge(-step)}
        className={`${btn} rounded-l-[5px]`}
      >
        <Icon name="minus" size={14} />
      </button>
      <span
        aria-live="polite"
        className="min-w-16 border-x border-line px-1.5 text-center leading-[30px] text-fg tabular-nums"
      >
        {format(current)}
        {unit ? <small className="ml-[3px] text-[12px] text-fg-mute">{unit}</small> : null}
      </span>
      <button
        type="button"
        aria-label="Increase"
        disabled={max !== undefined && current >= max}
        onClick={() => nudge(step)}
        className={`${btn} rounded-r-[5px]`}
      >
        <Icon name="plus" size={14} />
      </button>
    </span>
  );
}
