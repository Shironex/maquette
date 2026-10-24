import { Icon } from '../../icons/Icon';
import { cn } from '../../lib/cn';
import { Tooltip } from './Tooltip';

export interface HelpMarkProps {
  /** The accessible name of the mark, for example "About objects". */
  label: string;
  /** The short explanation shown on hover and on keyboard focus. */
  tip: string;
  side?: 'top' | 'bottom';
  className?: string;
}

/** A small question mark next to a label; the tip shows on hover and on focus. */
export function HelpMark({ label, tip, side = 'top', className }: HelpMarkProps) {
  return (
    <Tooltip label={tip} side={side} wrap>
      <button
        type="button"
        aria-label={label}
        className={cn(
          'inline-flex size-4 cursor-help items-center justify-center rounded-pill text-fg-mute transition-colors duration-120 hover:text-fg focus-visible:text-fg focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-clay',
          className
        )}
      >
        <Icon name="question" size={14} />
      </button>
    </Tooltip>
  );
}
