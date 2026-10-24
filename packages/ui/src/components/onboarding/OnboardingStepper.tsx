import { Icon } from '../../icons/Icon';
import { cn } from '../../lib/cn';

export interface OnboardingStepperProps {
  steps: readonly string[];
  /** Zero-based index of the step on screen. */
  current: number;
  /** Lets a done step be revisited. Steps ahead stay locked. */
  onSelect?: (index: number) => void;
  className?: string;
}

export function OnboardingStepper({ steps, current, onSelect, className }: OnboardingStepperProps) {
  return (
    <ol
      aria-label="Setup steps"
      className={cn('m-0 flex list-none flex-col gap-0.5 p-0', className)}
    >
      {steps.map((label, i) => {
        const state = i < current ? 'done' : i === current ? 'current' : 'todo';
        const row = cn(
          'flex h-11 w-full items-center gap-3 rounded-card px-3 text-left font-medium text-fg-mute',
          state === 'done' && 'text-fg-dim',
          state === 'current' && 'bg-clay-soft text-fg'
        );
        const mark = (
          <span
            className={cn(
              'grid size-6 flex-none place-items-center rounded-full border-[1.5px] border-line-strong font-mono text-[11.5px] leading-none font-semibold',
              state === 'done' && 'border-transparent bg-sage-soft text-sage',
              state === 'current' && 'border-clay-text text-clay-text'
            )}
          >
            {state === 'done' ? <Icon name="check" size={12} title="Done" /> : i + 1}
          </span>
        );
        return (
          <li key={label} aria-current={state === 'current' ? 'step' : undefined}>
            {state === 'done' && onSelect ? (
              <button
                type="button"
                onClick={() => onSelect(i)}
                className={cn(row, 'cursor-pointer hover:bg-hover')}
              >
                {mark}
                {label}
              </button>
            ) : (
              <div className={row}>
                {mark}
                {label}
              </div>
            )}
          </li>
        );
      })}
    </ol>
  );
}
