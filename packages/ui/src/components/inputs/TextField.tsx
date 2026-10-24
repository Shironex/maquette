import { useId, type InputHTMLAttributes, type ReactNode } from 'react';
import { Icon, type IconName } from '../../icons/Icon';
import { cn } from '../../lib/cn';
import { Field, INPUT_INNER, INPUT_SHELL, type FieldProps } from './Field';

export interface TextFieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'size'> {
  label?: string;
  hint?: ReactNode;
  note?: FieldProps['note'];
  icon?: IconName;
  mono?: boolean;
  unit?: string;
  size?: 'md' | 'lg';
  fieldClassName?: string;
}

export function TextField({
  label,
  hint,
  note,
  icon,
  mono,
  unit,
  size = 'md',
  id,
  className,
  fieldClassName,
  ...rest
}: TextFieldProps) {
  const auto = useId();
  const inputId = id ?? auto;
  return (
    <Field id={inputId} label={label} hint={hint} note={note} className={fieldClassName}>
      <span className={cn(INPUT_SHELL, size === 'lg' && 'h-10', className)}>
        {icon ? <Icon name={icon} size={14} className="text-fg-mute" /> : null}
        <input
          id={inputId}
          type="text"
          spellCheck={false}
          aria-describedby={hint || note ? `${inputId}-note` : undefined}
          className={cn(INPUT_INNER, mono && 'font-mono text-[12.5px] tracking-[-0.01em]')}
          {...rest}
        />
        {unit ? <span className="text-meta text-fg-mute">{unit}</span> : null}
      </span>
    </Field>
  );
}
