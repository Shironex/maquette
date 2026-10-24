import { useId, type ReactNode } from 'react';
import { cn } from '../../lib/cn';
import { Button } from '../actions/Button';
import { IconButton } from '../actions/IconButton';
import { Field, INPUT_INNER, INPUT_SHELL, type FieldProps } from './Field';

export interface PathFieldProps {
  label?: string;
  value: string;
  placeholder?: string;
  onChange?: (value: string) => void;
  /** Called on blur or Enter, for checks that should not run per keystroke. */
  onCommit?: (value: string) => void;
  onBrowse?: () => void;
  browseLabel?: string;
  /** Shows only a folder icon for Browse; the label becomes its accessible name. */
  compactBrowse?: boolean;
  note?: FieldProps['note'];
  hint?: ReactNode;
  size?: 'md' | 'lg';
  id?: string;
  className?: string;
}

/** A path, its Browse button and the detection note under it. */
export function PathField({
  label,
  value,
  placeholder,
  onChange,
  onCommit,
  onBrowse,
  browseLabel = 'Browse',
  compactBrowse,
  note,
  hint,
  size = 'md',
  id,
  className,
}: PathFieldProps) {
  const auto = useId();
  const inputId = id ?? auto;
  return (
    <Field id={inputId} label={label} hint={hint} note={note} className={className}>
      <div className="flex items-center gap-2">
        <span className={cn(INPUT_SHELL, size === 'lg' && 'h-10')}>
          <input
            id={inputId}
            value={value}
            placeholder={placeholder}
            readOnly={!onChange}
            spellCheck={false}
            autoComplete="off"
            aria-describedby={note || hint ? `${inputId}-note` : undefined}
            onChange={e => onChange?.(e.target.value)}
            onBlur={e => onCommit?.(e.target.value)}
            onKeyDown={e => {
              if (e.key === 'Enter') onCommit?.(e.currentTarget.value);
            }}
            className={cn(INPUT_INNER, 'font-mono text-[12.5px] tracking-[-0.01em]')}
          />
        </span>
        {onBrowse ? (
          compactBrowse ? (
            <IconButton
              icon="folder-open"
              label={browseLabel}
              variant="secondary"
              onClick={onBrowse}
            />
          ) : (
            <Button icon="folder-open" size={size === 'lg' ? 'lg' : 'md'} onClick={onBrowse}>
              {browseLabel}
            </Button>
          )
        ) : null}
      </div>
    </Field>
  );
}
