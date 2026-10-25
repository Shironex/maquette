import { cn } from '../../lib/cn';

export interface KbdProps {
  /** One key ("W"), or the keys of a chord pressed together (["⌘", "Z"]). */
  keys: string | readonly string[];
  /** `inverse` sits on the light tooltip bubble. */
  tone?: 'default' | 'inverse';
  className?: string;
}

const CAP = 'min-w-[18px] rounded-[5px] border px-1 text-center font-sans text-[11px] leading-4';

const TONE = {
  default: 'border-line text-fg-mute',
  inverse: 'border-app/25 text-app/75',
};

/** Keycaps for a keyboard shortcut, one cap per key. */
export function Kbd({ keys, tone = 'default', className }: KbdProps) {
  const list = typeof keys === 'string' ? [keys] : keys;
  const caps = list.map(key => (
    <kbd key={key} className={cn(CAP, TONE[tone])}>
      {key}
    </kbd>
  ));
  return <kbd className={cn('inline-flex flex-none items-center gap-0.5', className)}>{caps}</kbd>;
}
