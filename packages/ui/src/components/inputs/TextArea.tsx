import { useId, type ReactNode, type TextareaHTMLAttributes } from 'react';
import { cn } from '../../lib/cn';
import { Field, type FieldProps } from './Field';

export interface TextAreaProps extends TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: string;
  hint?: ReactNode;
  note?: FieldProps['note'];
  fieldClassName?: string;
}

/** A multi line text input in the same shell as `TextField`. */
export function TextArea({
  label,
  hint,
  note,
  id,
  rows = 5,
  className,
  fieldClassName,
  ...rest
}: TextAreaProps) {
  const auto = useId();
  const inputId = id ?? auto;
  return (
    <Field id={inputId} label={label} hint={hint} note={note} className={fieldClassName}>
      <textarea
        id={inputId}
        rows={rows}
        aria-describedby={hint || note ? `${inputId}-note` : undefined}
        className={cn(
          'w-full resize-none rounded-control border border-line-strong bg-app px-2.5 py-[9px] text-[13px] leading-[19px] text-fg outline-none transition-[border-color,box-shadow] duration-120 placeholder:text-fg-mute',
          'focus:border-clay focus:shadow-[0_0_0_3px_var(--clay-soft)] disabled:opacity-60',
          className
        )}
        {...rest}
      />
    </Field>
  );
}
