import { Icon } from '../../icons/Icon';
import { cn } from '../../lib/cn';
import { useMounted } from './Loader.hooks';

/** The one recipe for waiting: a verb, how far when that is known, how long it took last time. */
export interface WaitProps {
  /** What is on its way, as a verb; a screen reader hears it, and it stays when motion is reduced. */
  label?: string;
  /** "3 of 13", only when the work reports a count. */
  count?: string | null;
  /** "last 12s", only when a past run measured it. Never a guess. */
  estimate?: string | null;
}

export interface LoaderProps extends WaitProps {
  className?: string;
}

function waitMeta(count?: string | null, estimate?: string | null): string {
  return [count, estimate].filter(Boolean).join(' · ');
}

/** The one spinner for anything still on its way: a view, its data or a 3D stage. */
export function Loader({ label = 'Loading...', count, estimate, className }: LoaderProps) {
  const meta = waitMeta(count, estimate);
  const say = useMounted();
  return (
    <div
      role="status"
      data-view-loader
      className={cn('flex flex-col items-center gap-2 text-meta text-fg-mute', className)}
    >
      <Icon name="circle-notch" size={20} spin className="text-clay-text" />
      {say ? <span className="text-center">{label}</span> : null}
      {say && meta ? <span className="font-mono tabular-nums">{meta}</span> : null}
    </div>
  );
}

export interface WaitLineProps extends WaitProps {
  label: string;
  className?: string;
}

/** The same recipe on one line, for a row or a heading with no room for the stacked loader. */
export function WaitLine({ label, count, estimate, className }: WaitLineProps) {
  const meta = waitMeta(count, estimate);
  const say = useMounted();
  return (
    <span
      role="status"
      className={cn('inline-flex min-w-0 items-center gap-1.5 text-meta text-fg-mute', className)}
    >
      <Icon name="circle-notch" size={13} spin className="flex-none text-clay-text" />
      {say ? <span className="min-w-0 truncate">{label}</span> : null}
      {say && meta ? <span className="flex-none font-mono tabular-nums">{meta}</span> : null}
    </span>
  );
}
