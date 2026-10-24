import type { ReactNode } from 'react';
import { Icon } from '../../icons/Icon';
import { cn } from '../../lib/cn';
import { useControllable } from '../../lib/useControllable';

export interface CheckboxProps {
  checked?: boolean;
  defaultChecked?: boolean;
  onChange?: (checked: boolean) => void;
  label?: ReactNode;
  className?: string;
}

export function Checkbox({
  checked,
  defaultChecked = false,
  onChange,
  label,
  className,
}: CheckboxProps) {
  const [on, set] = useControllable(checked, defaultChecked, onChange);
  return (
    <label className={cn('inline-flex cursor-pointer items-center gap-2 text-fg', className)}>
      <input
        type="checkbox"
        checked={on}
        onChange={e => set(e.target.checked)}
        className="peer sr-only"
      />
      <span
        aria-hidden="true"
        className={cn(
          'inline-grid size-4 flex-none place-items-center rounded-[4px] border-[1.5px] peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-focus',
          on
            ? 'border-clay-text bg-clay text-clay-ink'
            : 'border-line-strong bg-raised text-transparent'
        )}
      >
        <Icon name="check" size={12} />
      </span>
      {label}
    </label>
  );
}
