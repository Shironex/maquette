import type { HTMLAttributes, ReactNode } from 'react';
import { cn } from '../../lib/cn';
import { Icon, type IconName } from '../../icons/Icon';
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
  /** Replaces the tris and size line, for example "No model yet". */
  meta?: string;
  status?: Status;
  /** Overrides the status word, for example "Waiting for your model". */
  statusLabel?: string;
  /** Overrides the status icon. */
  statusIcon?: IconName;
  progress?: number;
  progressLabel?: string;
  selected?: boolean;
  /** What stands in while there is no thumbnail: a kind icon and a word. */
  placeholder?: { icon: IconName; label?: string };
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
  meta,
  status,
  statusLabel,
  statusIcon,
  progress,
  progressLabel,
  selected,
  placeholder,
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
        className={cn(
          'relative block h-[116px] w-full cursor-pointer overflow-hidden rounded-t-card',
          thumb ? 'bg-studio' : 'border-b border-line bg-raised'
        )}
      >
        {thumb ? (
          <img
            src={thumb}
            alt=""
            draggable={false}
            className={cn(
              'block size-full',
              fit === 'cover'
                ? 'object-cover'
                : // A studio render's floor fades into the studio fill at the sides.
                  'object-contain [mask-image:linear-gradient(to_right,transparent,black_22%,black_78%,transparent)]'
            )}
          />
        ) : (
          <span className="flex size-full flex-col items-center justify-center gap-1.5 text-fg-mute">
            <Icon name={placeholder?.icon ?? 'cube'} size={22} />
            {placeholder?.label ? <span className="text-meta">{placeholder.label}</span> : null}
          </span>
        )}
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
          {meta ?? [tris ? `${tris} tris` : null, size].filter(Boolean).join(' · ')}
        </div>
        {running && progress !== undefined ? (
          <ProgressBar value={progress} label={progressLabel ?? 'Running'} className="mt-1" />
        ) : status ? (
          <div className="mt-1">
            <StatusChip status={status} label={statusLabel} icon={statusIcon} />
          </div>
        ) : null}
      </div>
      {moreOpen && menu ? <div className="absolute top-full right-1 z-20 -mt-8">{menu}</div> : null}
    </div>
  );
}
