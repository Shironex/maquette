import type { ReactNode } from 'react';
import { Icon } from '../../icons/Icon';
import { cn } from '../../lib/cn';

export type FieldTone = 'found' | 'warning' | 'missing';

export interface FieldProps {
  id: string;
  label?: string;
  hint?: ReactNode;
  /** A detection or validation note under the control, with its icon. */
  note?: { tone: FieldTone; text: ReactNode };
  /** Label beside the control on one line, for dense rows. */
  inline?: boolean;
  className?: string;
  children: ReactNode;
}

const NOTE_ICON = { found: 'check-circle', warning: 'warning', missing: 'warning-circle' } as const;
const NOTE_COLOR = { found: 'text-sage', warning: 'text-amber', missing: 'text-danger' } as const;

/** Label, control and one line under it. Inputs inside take the `id`. */
export function Field({ id, label, hint, note, inline, className, children }: FieldProps) {
  return (
    <div
      className={cn(
        'flex min-w-0',
        inline ? 'flex-none flex-row items-center gap-2' : 'flex-col gap-1.5',
        className
      )}
    >
      {label ? (
        <label
          htmlFor={id}
          className="text-[12px] leading-4 font-medium whitespace-nowrap text-fg-dim"
        >
          {label}
        </label>
      ) : null}
      {children}
      {note ? (
        <p
          id={`${id}-note`}
          className={cn('flex items-center gap-1.5 text-meta', NOTE_COLOR[note.tone])}
        >
          <Icon name={NOTE_ICON[note.tone]} size={14} />
          {note.text}
        </p>
      ) : hint ? (
        <p id={`${id}-note`} className="text-meta text-fg-mute">
          {hint}
        </p>
      ) : null}
    </div>
  );
}

export const INPUT_SHELL =
  'flex h-[34px] w-full items-center gap-2 rounded-control border border-line-strong bg-app px-2.5 text-fg transition-[border-color,box-shadow] duration-120 ' +
  'focus-within:border-clay focus-within:shadow-[0_0_0_3px_var(--clay-soft)] has-[input:disabled]:opacity-60';

export const INPUT_INNER =
  'h-full min-w-0 flex-1 border-0 bg-transparent p-0 text-[13px] text-fg outline-none placeholder:text-fg-mute';
