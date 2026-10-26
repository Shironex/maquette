import { Icon, cn, type IconName } from '@maquette/ui';
import type { RepoLine } from '@/state/repo-lines';

const LOOK: Record<RepoLine['tone'], { icon: IconName; className: string }> = {
  found: { icon: 'check-circle', className: 'text-sage' },
  warning: { icon: 'warning', className: 'text-amber' },
  missing: { icon: 'warning-circle', className: 'text-danger' },
  info: { icon: 'info', className: 'text-fg-dim' },
};

export interface RepoLinesProps {
  lines: readonly RepoLine[];
  label: string;
}

/** What the repo check found, one line each. */
export default function RepoLines({ lines, label }: RepoLinesProps) {
  if (!lines.length) return null;
  const items = lines.map(line => {
    const look = LOOK[line.tone];
    return (
      <li key={line.text} className="flex flex-wrap items-start gap-x-1.5 text-meta text-fg-mute">
        <span className={cn('inline-flex items-start gap-1.5', look.className)}>
          <Icon name={look.icon} size={14} className="mt-px" />
          {line.text}
        </span>
        {line.detail ? <span className="min-w-0">{line.detail}</span> : null}
      </li>
    );
  });
  return (
    <ul aria-label={label} className="mt-2 flex flex-col gap-1">
      {items}
    </ul>
  );
}
