import type { HTMLAttributes, ReactNode } from 'react';
import { cn } from '../../lib/cn';
import { IconButton } from '../actions/IconButton';
import { ProgressBar } from '../status/ProgressBar';
import { StatusChip, type Status } from '../status/StatusChip';
import { VersionBadge } from '../status/VersionBadge';

export interface AssetCardProps extends Omit<HTMLAttributes<HTMLDivElement>, 'onSelect'> {
  name: string;
  thumb?: string;
  fit?: 'contain' | 'cover';
  version?: number | string;
  latest?: boolean;
  tris?: string;
  size?: string;
  status?: Status;
  progress?: number;
  progressLabel?: string;
  selected?: boolean;
  onSelect?: () => void;
  /** Opens the card's menu from the dots button. */
  onMore?: () => void;
  /** True while that menu is open. */
  moreOpen?: boolean;
  /** The menu itself, placed under the dots button. */
  menu?: ReactNode;
  className?: string;
}

export function AssetCard({
  name,
  thumb,
  fit = 'contain',
  version,
  latest,
  tris,
  size,
  status,
  progress,
  progressLabel,
  selected,
  onSelect,
  onMore,
  moreOpen,
  menu,
  className,
  ...rest
}: AssetCardProps) {
  const running = status === 'running';
  return (
    <div
      {...rest}
      className={cn(
        'relative flex flex-col rounded-card border border-line bg-raised transition-colors duration-120 hover:border-line-strong',
        selected && 'border-clay-line',
        running && 'border-amber/50 shadow-running',
        className
      )}
    >
      <button
        type="button"
        aria-pressed={selected}
        aria-label={`${name}${version !== undefined ? `, v${version}` : ''}`}
        onClick={onSelect}
        className="relative block h-[116px] w-full cursor-pointer overflow-hidden rounded-t-card bg-studio"
      >
        {thumb ? (
          <img
            src={thumb}
            alt=""
            className={cn(
              'block size-full',
              fit === 'cover' ? 'object-cover' : 'object-contain brightness-[0.84] saturate-[1.04]'
            )}
          />
        ) : null}
        {version !== undefined ? (
          <span className="absolute top-2 right-2">
            <VersionBadge version={version} latest={latest} onImage />
          </span>
        ) : null}
      </button>
      <div className="flex flex-col gap-1 px-2.5 pt-[9px] pb-2.5">
        <div className="flex items-center justify-between gap-1.5 font-semibold">
          <span className="truncate">{name}</span>
          <IconButton
            icon="dots-three"
            label={`More for ${name}`}
            size="sm"
            aria-haspopup={menu !== undefined ? 'menu' : undefined}
            aria-expanded={menu !== undefined ? Boolean(moreOpen) : undefined}
            onClick={onMore}
          />
        </div>
        <div className="font-mono text-[11.5px] leading-[15px] text-fg-mute">
          {[tris ? `${tris} tris` : null, size].filter(Boolean).join(' · ')}
        </div>
        {running && progress !== undefined ? (
          <ProgressBar value={progress} label={progressLabel ?? 'Running'} className="mt-1" />
        ) : status ? (
          <div className="mt-1">
            <StatusChip status={status} />
          </div>
        ) : null}
      </div>
      {moreOpen && menu ? <div className="absolute top-full right-1 z-20 -mt-8">{menu}</div> : null}
    </div>
  );
}
