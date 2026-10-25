import { useId, type InputHTMLAttributes } from 'react';
import { cn } from '../../lib/cn';
import { Field, INPUT_INNER, INPUT_SHELL, type FieldProps } from './Field';
import { useNumberText } from './NumberField.hooks';

export interface NumberFieldProps
  extends
    Pick<FieldProps, 'label' | 'hint' | 'note' | 'inline'>,
    Omit<
      InputHTMLAttributes<HTMLInputElement>,
      'value' | 'onChange' | 'size' | 'min' | 'max' | 'prefix'
    > {
  value: number;
  onChange: (value: number) => void;
  unit?: string;
  min?: number;
  max?: number;
  /** How the value reads while the field is not being edited. */
  format?: (value: number) => string;
  /** Turns typed text into a number; returns null for text it cannot read. */
  parse?: (text: string) => number | null;
  size?: 'sm' | 'md';
  /** Fixed text before the value, such as `127.0.0.1:`. */
  prefix?: string;
  fieldClassName?: string;
}

export function defaultParse(text: string): number | null {
  const cleaned = text.replace(/[,\s_]/g, '');
  if (!/^-?\d*\.?\d+$/.test(cleaned)) return null;
  return Number(cleaned);
}

/**
 * A number with its unit. The text is free while focused and is committed,
 * clamped, on blur or Enter; text it cannot read reverts to the last value.
 */
export function NumberField({
  label,
  hint,
  value,
  onChange,
  unit,
  min,
  max,
  format = String,
  parse = defaultParse,
  size = 'md',
  prefix,
  inline,
  note,
  id,
  className,
  fieldClassName,
  ...rest
}: NumberFieldProps) {
  const auto = useId();
  const inputId = id ?? auto;
  const { text, setText, setEditing, commit } = useNumberText({
    value,
    onChange,
    min,
    max,
    format,
    parse,
  });

  return (
    <Field
      id={inputId}
      label={label}
      hint={hint}
      note={note}
      inline={inline}
      className={fieldClassName}
    >
      <span className={cn(INPUT_SHELL, size === 'sm' && 'h-7', prefix && 'gap-0', className)}>
        {prefix ? (
          <span className="font-mono text-[12.5px] tracking-[-0.01em] text-fg-mute">{prefix}</span>
        ) : null}
        <input
          {...rest}
          id={inputId}
          inputMode="decimal"
          aria-describedby={note || hint ? `${inputId}-note` : undefined}
          value={text}
          onFocus={() => setEditing(true)}
          onChange={e => setText(e.target.value)}
          onBlur={commit}
          onKeyDown={e => {
            if (e.key === 'Enter') e.currentTarget.blur();
          }}
          className={cn(
            INPUT_INNER,
            'font-mono tabular-nums tracking-[-0.01em]',
            size === 'sm' && 'text-[12px]'
          )}
        />
        {unit ? <span className="text-[12px] text-fg-mute">{unit}</span> : null}
      </span>
    </Field>
  );
}
