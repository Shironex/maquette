import { Icon, IconButton, ProgressBar, StatusChip, Tooltip, VersionBadge, cn } from '@maquette/ui';
import { LIST_COLUMNS } from '../columns';
import { CardMenu } from '../../BoardCard/CardMenu';
import { useListRow } from './ListRow.hooks';
import type { ListRowProps } from './ListRow.types';

const CELL = 'h-11 px-2 align-middle';
/** A cell drops out with its column heading on a narrow window. */
const narrow = (key: string) => LIST_COLUMNS.find(c => c.key === key)?.narrow;

/** One asset in the Board's list: the card's numbers in a row, its box and its menu. */
export default function ListRow(props: ListRowProps) {
  const { row, board, tabStop, menuUp, onToggleMenu, onFocus, position } = props;
  const { asset, n } = row;
  const v = useListRow(props);
  const { selection, dropJob } = board;

  return (
    <tr
      {...position}
      data-row-id={asset.id}
      data-drop-job={v.waiting?.id}
      tabIndex={tabStop ? 0 : -1}
      aria-selected={v.checked}
      aria-label={asset.name}
      onFocus={e => {
        if (e.target === e.currentTarget) onFocus(asset.id);
      }}
      onClickCapture={selection.shiftClick(asset.id)}
      className={cn(
        'group/row border-b border-line text-[13px] transition-colors duration-120 last:border-b-0 hover:bg-hover/60 focus-visible:bg-hover focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-focus',
        v.checked && 'bg-clay-soft/35 hover:bg-clay-soft/50',
        v.waiting && dropJob === v.waiting.id && 'bg-clay-soft/50'
      )}
    >
      <td className={cn(CELL, 'w-[36px] pr-0 pl-3')}>
        <Tooltip label="Select for a batch" detail="Shift-click selects the rows between">
          <button
            type="button"
            role="checkbox"
            tabIndex={-1}
            aria-checked={v.checked}
            aria-label={`Select ${asset.name}`}
            onClick={e => selection.check(asset.id, !v.checked, e.shiftKey)}
            className={cn(
              'grid size-[18px] cursor-pointer place-items-center rounded-[5px] border-[1.5px] transition-colors duration-120',
              v.checked
                ? 'border-clay-text bg-clay text-clay-ink'
                : 'border-line-strong bg-raised text-transparent hover:border-fg-mute'
            )}
          >
            <Icon name="check" size={12} />
          </button>
        </Tooltip>
      </td>
      <td className={cn(CELL, 'w-[60px]')}>
        <span
          className={cn(
            'grid h-[30px] w-[44px] place-items-center overflow-hidden rounded-[6px] border border-line text-fg-mute',
            v.thumb ? 'bg-studio' : 'bg-raised'
          )}
        >
          {v.thumb ? (
            <img src={v.thumb} alt="" draggable={false} className="size-full object-contain" />
          ) : (
            <Icon name={v.kindIcon} size={15} />
          )}
        </span>
      </td>
      <td className={cn(CELL, 'max-w-0')}>
        <span className="block truncate font-semibold">{asset.name}</span>
      </td>
      <td className={cn(CELL, 'text-fg-dim')}>
        <span className="inline-flex items-center gap-1.5" title={v.kindLabel}>
          <Icon name={v.kindIcon} size={14} className="text-fg-mute" />
          <span className="max-[1279px]:sr-only">{v.kindLabel}</span>
        </span>
      </td>
      <td className={cn(CELL, 'text-fg-dim')}>
        <span className="inline-flex items-center gap-1.5">
          <Icon name={v.stageIcon} size={14} className="text-fg-mute" />
          {v.stageLabel}
        </span>
      </td>
      <td className={cn(CELL, narrow('version'))}>
        {n.version !== undefined ? (
          <VersionBadge version={n.version} latest />
        ) : (
          <span className="text-meta text-fg-mute">No model</span>
        )}
      </td>
      <td
        className={cn(
          CELL,
          'text-right font-mono text-[12px] text-fg-dim',
          narrow('tris'),
          v.over && 'text-amber'
        )}
        title={v.over ?? undefined}
      >
        {v.budgetLine ?? n.tris}
      </td>
      <td
        className={cn(
          CELL,
          'text-right font-mono text-[12px] whitespace-nowrap text-fg-dim',
          narrow('size')
        )}
      >
        {row.size}
        {v.sizeNote ? (
          <span className="ml-1.5 font-sans text-[10.5px] text-fg-mute">{v.sizeNote}</span>
        ) : null}
      </td>
      <td className={cn(CELL, 'pl-4')}>
        {v.progress !== undefined ? (
          <ProgressBar value={v.progress} label={v.sent?.progressLabel ?? 'Running'} />
        ) : (
          <StatusChip
            status={v.over && !v.sent ? 'review' : v.status}
            label={v.over && !v.sent ? 'Over budget' : v.sent?.label}
            icon={v.over && !v.sent ? 'warning' : v.sent?.icon}
          />
        )}
      </td>
      <td className={cn(CELL, 'text-meta whitespace-nowrap text-fg-mute', narrow('changed'))}>
        {v.changed}
      </td>
      <td className={cn(CELL, 'relative w-[44px] pr-2 text-right')}>
        <IconButton
          icon="dots-three"
          label={`More for ${asset.name}`}
          size="sm"
          tabIndex={-1}
          aria-haspopup="menu"
          aria-expanded={v.menuOpen}
          onClick={() => onToggleMenu(asset.id)}
        />
        {v.menuOpen ? (
          <div
            className={cn(
              'absolute right-2 z-20 text-left',
              menuUp ? 'bottom-full mb-1' : 'top-full -mt-1'
            )}
          >
            <CardMenu asset={asset} waiting={v.waiting} refPicture={v.refPicture} board={board} />
          </div>
        ) : null}
      </td>
    </tr>
  );
}
