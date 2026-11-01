import type { HTMLAttributes, ReactNode } from 'react';
import { cn } from '../../lib/cn';
import { Icon, type IconName } from '../../icons/Icon';
import { IconButton } from '../actions/IconButton';
import { Tooltip } from '../feedback/Tooltip';
import { ProgressBar } from '../status/ProgressBar';
import { StatusChip, type Status } from '../status/StatusChip';
import { VersionBadge } from '../status/VersionBadge';

/** The picture: a thumbnail, or a kind icon and a word while there is none. */
export interface AssetCardPicture {
  thumb?: string;
  fit?: 'contain' | 'cover';
  /** What stands in while there is no thumbnail: a kind icon and a word; `busy` while one renders. */
  placeholder?: { icon: IconName; label?: string; busy?: boolean };
}

/** The line under the name. */
export interface AssetCardDetails {
  tris?: string;
  size?: string;
  /** Replaces the tris and size line, for example "No model yet". */
  meta?: string;
}

/** The status chip, or the progress bar while running. */
export interface AssetCardStatus {
  state?: Status;
  /** Overrides the status word, for example "Waiting for your model". */
  label?: string;
  /** Overrides the status icon. */
  icon?: IconName;
  progress?: number;
  progressLabel?: string;
  /** The whole status in a tooltip on the chip, for a word that may not fit. */
  tooltip?: string;
  /** A small line under the chip, for example the name a By hand send waits for. */
  note?: ReactNode;
}

/** The dots button and the menu it opens. */
export interface AssetCardMore {
  /** Opens the card's menu from the dots button. */
  onOpen?: () => void;
  /** True while that menu is open. */
  open?: boolean;
  /** The menu itself, placed under the dots button. */
  menu?: ReactNode;
}

/** The select box on the picture, shown when this is set. */
export interface AssetCardSelection {
  /** True while the card is in the selection. */
  checked: boolean;
  /** The select box was clicked; `range` is true with Shift held. */
  onCheck?: (checked: boolean, range: boolean) => void;
  /** Keeps the select box on show, for while a selection is under way. */
  selecting?: boolean;
}

export interface AssetCardProps extends Omit<HTMLAttributes<HTMLDivElement>, 'onSelect'> {
  name: string;
  picture?: AssetCardPicture;
  version?: number | string;
  latest?: boolean;
  details?: AssetCardDetails;
  status?: AssetCardStatus;
  selected?: boolean;
  onSelect?: () => void;
  more?: AssetCardMore;
  selection?: AssetCardSelection;
  /** Actions under the status, for example "Add model...". */
  actions?: ReactNode;
  className?: string;
}

export function AssetCard({
  name,
  picture = {},
  version,
  latest,
  details = {},
  status: statusInfo = {},
  selected,
  onSelect,
  more = {},
  selection,
  actions,
  className,
  ...rest
}: AssetCardProps) {
  const { thumb, fit = 'contain', placeholder } = picture;
  const { tris, size, meta } = details;
  const {
    state: status,
    label: statusLabel,
    icon: statusIcon,
    progress,
    progressLabel,
    tooltip: statusTooltip,
    note,
  } = statusInfo;
  const { onOpen: onMore, open: moreOpen, menu } = more;
  const checked = selection?.checked;
  const onCheck = selection?.onCheck;
  const selecting = selection?.selecting;
  const metaLine = meta ?? [tris ? `${tris} tris` : null, size].filter(Boolean).join(' · ');
  const running = status === 'running';
  const selectable = selection !== undefined;
  return (
    <div
      {...rest}
      className={cn(
        'group/card relative flex flex-col rounded-card border border-line bg-raised transition-colors duration-120 hover:border-line-strong',
        (selected || checked) && 'border-clay-line',
        checked && 'ring-2 ring-clay/35',
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
            <Icon
              name={placeholder?.icon ?? 'cube'}
              size={22}
              className={cn(placeholder?.busy && 'animate-pulse motion-reduce:animate-none')}
            />
            {placeholder?.label ? <span className="text-meta">{placeholder.label}</span> : null}
          </span>
        )}
        {version !== undefined ? (
          <span className="absolute top-2 right-2">
            <VersionBadge version={version} latest={latest} onImage />
          </span>
        ) : null}
      </button>
      {selectable ? (
        <Tooltip label="Select for a batch" detail="Shift-click selects the cards between">
          <button
            type="button"
            role="checkbox"
            aria-checked={checked}
            aria-label={`Select ${name}`}
            onClick={e => onCheck?.(!checked, e.shiftKey)}
            className={cn(
              'absolute top-2 left-2 grid size-6 cursor-pointer place-items-center rounded-[6px] border-[1.5px] shadow-[0_1px_3px_rgba(0,0,0,0.35)] transition-opacity duration-120 focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus',
              checked
                ? 'border-clay-text bg-clay text-clay-ink'
                : 'border-line-strong bg-panel/90 text-transparent hover:border-fg-mute',
              checked || selecting ? 'opacity-100' : 'opacity-0 group-hover/card:opacity-100'
            )}
          >
            <Icon name="check" size={13} />
          </button>
        </Tooltip>
      ) : null}
      <div className="flex flex-col gap-1 px-2.5 pt-[9px] pb-2.5">
        <div className="flex items-start justify-between gap-1.5 font-semibold">
          <span className="line-clamp-2 min-w-0 [overflow-wrap:anywhere]" title={name}>
            {name}
          </span>
          <IconButton
            icon="dots-three"
            label={`More for ${name}`}
            size="sm"
            aria-haspopup={menu !== undefined ? 'menu' : undefined}
            aria-expanded={menu !== undefined ? Boolean(moreOpen) : undefined}
            // A 40 px target around the 26 px button, without moving the card's lines.
            className="relative before:absolute before:-inset-[7px] before:content-['']"
            onClick={onMore}
          />
        </div>
        <div className="font-mono text-[11.5px] leading-[15px] text-fg-mute">{metaLine}</div>
        {running && progress !== undefined ? (
          <ProgressBar value={progress} label={progressLabel ?? 'Running'} className="mt-1" />
        ) : status ? (
          <div className="mt-1 min-w-0">
            <Tooltip label={statusTooltip} wrap disabled={!statusTooltip}>
              <StatusChip status={status} label={statusLabel} icon={statusIcon} />
            </Tooltip>
          </div>
        ) : null}
        {note ? <div className="min-w-0">{note}</div> : null}
        {actions ? <div className="@container mt-1.5 flex flex-wrap gap-1.5">{actions}</div> : null}
      </div>
      {moreOpen && menu ? <div className="absolute top-full right-1 z-20 -mt-8">{menu}</div> : null}
    </div>
  );
}
