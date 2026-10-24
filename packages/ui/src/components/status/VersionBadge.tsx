import { cn } from '../../lib/cn';

export interface VersionBadgeProps {
  version: number | string;
  latest?: boolean;
  onImage?: boolean;
  className?: string;
}

export function VersionBadge({ version, latest, onImage, className }: VersionBadgeProps) {
  const v = String(version);
  return (
    <span
      title={latest ? 'Latest version' : undefined}
      className={cn(
        'inline-flex h-[18px] items-center rounded-[5px] px-1.5 font-mono text-[11px] leading-none font-medium',
        latest ? 'bg-clay-soft text-clay-text' : 'bg-hover text-fg-dim',
        onImage && (latest ? 'bg-clay text-clay-ink' : 'bg-scrim text-fg'),
        className
      )}
    >
      {v.startsWith('v') ? v : `v${v}`}
    </span>
  );
}
