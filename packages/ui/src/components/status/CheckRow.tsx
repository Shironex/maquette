import type { ReactNode } from 'react';
import { Icon } from '../../icons/Icon';
import { cn } from '../../lib/cn';

export type CheckStatus = 'pass' | 'warning' | 'fail';

const ICON = { pass: 'check-circle', warning: 'warning', fail: 'x-circle' } as const;
const TONE = { pass: 'text-sage', warning: 'text-amber', fail: 'text-danger' } as const;
const WORD = { pass: 'Passed', warning: 'Warning', fail: 'Failed' } as const;

export interface CheckRowProps {
  status: CheckStatus;
  title: string;
  detail?: string;
  action?: ReactNode;
  className?: string;
}

export function CheckRow({ status, title, detail, action, className }: CheckRowProps) {
  return (
    <div
      className={cn(
        'grid grid-cols-[20px_1fr_auto] items-start gap-3 rounded-card bg-raised p-3',
        className
      )}
    >
      <Icon
        name={ICON[status]}
        size={18}
        title={WORD[status]}
        className={cn('mt-px', TONE[status])}
      />
      <div>
        <div className="font-medium text-fg">{title}</div>
        {detail ? (
          <div className="mt-0.5 text-[12px] leading-[17px] text-fg-dim">{detail}</div>
        ) : null}
      </div>
      {action ?? null}
    </div>
  );
}
