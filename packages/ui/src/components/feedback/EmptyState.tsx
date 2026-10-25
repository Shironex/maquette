import type { ReactNode } from 'react';
import { Icon, type IconName } from '../../icons/Icon';
import { cn } from '../../lib/cn';

export interface EmptyStateStep {
  title: string;
  detail?: string;
}

export interface EmptyStateProps {
  icon?: IconName;
  title: string;
  body?: string;
  steps?: readonly EmptyStateStep[];
  action?: ReactNode;
  className?: string;
}

/** An empty view that teaches the next step instead of apologising. */
export function EmptyState({
  icon = 'cube',
  title,
  body,
  steps,
  action,
  className,
}: EmptyStateProps) {
  const stepItems = steps?.map((st, i) => (
    <li key={st.title} className="grid grid-cols-[24px_1fr] items-start gap-3 text-fg">
      <span className="inline-flex size-6 items-center justify-center rounded-full border border-line-strong font-mono text-[12px] font-semibold text-fg-dim">
        {i + 1}
      </span>
      <span>
        {st.title}
        {st.detail ? <small className="block text-meta text-fg-mute">{st.detail}</small> : null}
      </span>
    </li>
  ));
  return (
    <div className={cn('flex max-w-[440px] flex-col items-start gap-4 p-8', className)}>
      <span className="inline-flex size-11 items-center justify-center rounded-card border border-line bg-raised text-clay-text">
        <Icon name={icon} size={22} />
      </span>
      <h2 className="m-0 text-panel-title text-fg">{title}</h2>
      {body ? <p className="-mt-2 mb-0 text-fg-dim">{body}</p> : null}
      {steps ? <ol className="m-0 flex w-full list-none flex-col gap-2 p-0">{stepItems}</ol> : null}
      {action ?? null}
    </div>
  );
}
