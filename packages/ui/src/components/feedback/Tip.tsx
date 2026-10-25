import type { ReactNode } from 'react';
import { Icon } from '../../icons/Icon';
import { cn } from '../../lib/cn';
import { IconButton } from '../actions/IconButton';

export interface TipProps {
  /** One short sentence. */
  children: ReactNode;
  /** An optional link-style action, for example "Show me". */
  action?: { label: string; onClick: () => void };
  /** Shows a dismiss button; the caller decides what dismissing remembers. */
  onDismiss?: () => void;
  className?: string;
}

/** A quiet hint row: an icon, one sentence, an optional action, and a way to dismiss it. */
export function Tip({ children, action, onDismiss, className }: TipProps) {
  return (
    <div
      role="note"
      className={cn(
        'flex items-center gap-2 rounded-control bg-raised py-1.5 pr-1.5 pl-2.5 text-meta text-fg-dim',
        className
      )}
    >
      <Icon name="info" size={14} className="text-fg-mute" />
      <span className="min-w-0 flex-1">{children}</span>
      {action ? (
        <button
          type="button"
          className="flex-none cursor-pointer text-clay-text underline-offset-2 hover:underline active:text-clay"
          onClick={action.onClick}
        >
          {action.label}
        </button>
      ) : null}
      {onDismiss ? <IconButton icon="x" size="sm" label="Dismiss tip" onClick={onDismiss} /> : null}
    </div>
  );
}
