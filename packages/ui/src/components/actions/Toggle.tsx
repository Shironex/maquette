import { useId } from 'react';
import { cn } from '../../lib/cn';
import { useControllable } from '../../lib/useControllable';

export interface ToggleProps {
  checked?: boolean;
  defaultChecked?: boolean;
  onChange?: (checked: boolean) => void;
  /** Visible label. Without one, pass `aria-label`. */
  label?: string;
  description?: string;
  disabled?: boolean;
  'aria-label'?: string;
  className?: string;
}

/** A switch. The input stays a real checkbox with role="switch" for keyboards and readers. */
export function Toggle({
  checked,
  defaultChecked = false,
  onChange,
  label,
  description,
  disabled,
  className,
  ...aria
}: ToggleProps) {
  const [on, set] = useControllable(checked, defaultChecked, onChange);
  const descId = useId();
  return (
    <label
      className={cn(
        'inline-flex cursor-pointer items-center gap-2 text-fg',
        description && 'items-start gap-3',
        disabled && 'cursor-not-allowed opacity-45',
        className
      )}
    >
      <input
        type="checkbox"
        role="switch"
        checked={on}
        disabled={disabled}
        aria-label={aria['aria-label']}
        aria-describedby={description ? descId : undefined}
        onChange={e => set(e.target.checked)}
        className="peer sr-only"
      />
      <span
        aria-hidden="true"
        className={cn(
          'relative h-[18px] w-8 flex-none rounded-pill transition-colors duration-140 ease-out peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-focus',
          on ? 'bg-clay' : 'bg-line-strong',
          description && 'mt-px'
        )}
      >
        <span
          className={cn(
            'absolute top-0.5 left-0.5 size-3.5 rounded-full transition-transform duration-160 ease-[cubic-bezier(.22,1,.36,1)]',
            on ? 'translate-x-3.5 bg-[#fbf4ee]' : 'bg-fg-dim'
          )}
        />
      </span>
      {label || description ? (
        <span className="flex flex-col">
          {label ? <span>{label}</span> : null}
          {description ? (
            <span id={descId} className="text-meta text-fg-mute">
              {description}
            </span>
          ) : null}
        </span>
      ) : null}
    </label>
  );
}
