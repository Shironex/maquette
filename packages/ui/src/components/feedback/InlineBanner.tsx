import type { ReactNode } from 'react';
import { Icon } from '../../icons/Icon';
import { cn } from '../../lib/cn';

export type BannerTone = 'danger' | 'warning' | 'success' | 'info';

const ICON = {
  danger: 'warning-circle',
  warning: 'warning',
  success: 'check-circle',
  info: 'info',
} as const;
const TONE: Record<BannerTone, string> = {
  danger: 'bg-danger-soft border-danger/28 [&>svg]:text-danger',
  warning: 'bg-amber-soft border-amber/28 [&>svg]:text-amber',
  success: 'bg-sage-soft border-sage/28 [&>svg]:text-sage',
  info: 'bg-panel border-line [&>svg]:text-fg-mute',
};

export interface InlineBannerProps {
  tone?: BannerTone;
  title: ReactNode;
  detail?: ReactNode;
  actions?: ReactNode;
  className?: string;
}

export function InlineBanner({
  tone = 'info',
  title,
  detail,
  actions,
  className,
}: InlineBannerProps) {
  return (
    <div
      role={tone === 'danger' ? 'alert' : 'status'}
      className={cn(
        'grid grid-cols-[18px_1fr_auto] items-start gap-2.5 rounded-card border px-3.5 py-3',
        TONE[tone],
        className
      )}
    >
      <Icon name={ICON[tone]} size={16} className="mt-0.5" />
      <div className="min-w-0">
        <div className={cn(detail ? 'font-medium text-fg' : 'text-fg-dim')}>{title}</div>
        {detail ? <div className="text-[12px] leading-[17px] text-fg-dim">{detail}</div> : null}
      </div>
      {actions ? <div className="flex gap-2">{actions}</div> : <span />}
    </div>
  );
}
