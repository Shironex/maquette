import { useId, type ReactNode, type SelectHTMLAttributes } from 'react';
import { Icon } from '../../icons/Icon';
import { cn } from '../../lib/cn';
import { Field, INPUT_SHELL } from './Field';

export interface SelectOption<T extends string> {
  value: T;
  label: string;
}

export interface SelectProps<T extends string> extends Omit<
  SelectHTMLAttributes<HTMLSelectElement>,
  'value' | 'onChange' | 'size'
> {
  label?: string;
  hint?: ReactNode;
  options: readonly (T | SelectOption<T>)[];
  value?: T;
  onChange?: (value: T) => void;
  size?: 'sm' | 'md';
  inline?: boolean;
  fieldClassName?: string;
}

export function Select<T extends string>({
  label,
  hint,
  options,
  value,
  onChange,
  size = 'md',
  inline,
  id,
  className,
  fieldClassName,
  ...rest
}: SelectProps<T>) {
  const auto = useId();
  const selectId = id ?? auto;
  return (
    <Field id={selectId} label={label} hint={hint} inline={inline} className={fieldClassName}>
      <span
        className={cn(INPUT_SHELL, 'relative pr-2', size === 'sm' && 'h-7 text-[12px]', className)}
      >
        <select
          id={selectId}
          value={value}
          onChange={e => onChange?.(e.target.value as T)}
          className="h-full min-w-0 flex-1 cursor-pointer appearance-none border-0 bg-transparent p-0 text-inherit text-fg outline-none [&>option]:bg-panel"
          {...rest}
        >
          {options.map(o => {
            const opt = typeof o === 'string' ? { value: o, label: o } : o;
            return (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            );
          })}
        </select>
        <Icon name="caret-down" size={14} className="pointer-events-none text-fg-mute" />
      </span>
    </Field>
  );
}
