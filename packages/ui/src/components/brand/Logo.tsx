import { cn } from '../../lib/cn';
import { Monogram } from './Monogram';

export interface LogoProps {
  size?: number;
  wordmark?: boolean;
  className?: string;
}

/** The monogram plus the lowercase wordmark. Never set the name another way. */
export function Logo({ size = 28, wordmark = true, className }: LogoProps) {
  return (
    <span className={cn('inline-flex items-center gap-2.5 text-fg', className)}>
      <Monogram size={size} />
      {wordmark ? <Wordmark size={Math.round(size * 0.62)} /> : null}
    </span>
  );
}

export function Wordmark({ size = 16, className }: { size?: number; className?: string }) {
  return (
    <span
      className={cn('font-semibold leading-none tracking-[-0.035em] text-fg', className)}
      style={{ fontSize: size }}
    >
      maquette
    </span>
  );
}
