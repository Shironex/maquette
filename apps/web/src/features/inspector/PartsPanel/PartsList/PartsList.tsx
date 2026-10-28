import { Button, Icon, IconButton, cn } from '@maquette/ui';
import type { PartRow } from '@/ipc/client';
import { formatK } from '@/lib/format';
import { MAX_PART_NAME } from '@/state/parts';
import { partColour } from '@/features/viewer/part-colours';
import { usePartRename } from './PartsList.hooks';

export interface PartsListProps {
  rows: readonly PartRow[];
  selected: readonly string[];
  onPick: (id: string, adding: boolean) => void;
  onRename: (id: string, name: string) => void;
  onUnmerge: (id: string) => void;
}

/** One part: its colour, the name it ships as, its triangles; double click renames. */
function PartItem({
  row,
  index,
  selected,
  onPick,
  onRename,
  onUnmerge,
}: { row: PartRow; index: number; selected: boolean } & Omit<PartsListProps, 'rows' | 'selected'>) {
  const { editing, draft, setDraft, start, finish } = usePartRename(row, onRename);
  const joined = row.joined.join(', ');
  return (
    <li
      className={cn(
        'flex h-8 items-center gap-2 rounded-[8px] px-2',
        selected ? 'bg-clay-soft' : 'hover:bg-hover',
        row.hidden && 'opacity-60'
      )}
    >
      <span
        aria-hidden
        className="size-2.5 flex-none rounded-full"
        style={{ background: partColour(index) }}
      />
      {editing ? (
        <input
          aria-label={`Rename ${row.name}`}
          autoFocus
          maxLength={MAX_PART_NAME}
          value={draft}
          onChange={e => setDraft(e.target.value)}
          onBlur={() => finish(true)}
          onKeyDown={e => {
            if (e.key === 'Enter') finish(true);
            if (e.key === 'Escape') finish(false);
          }}
          className="h-6 min-w-0 flex-1 rounded-[6px] border border-line bg-panel px-1.5 font-mono text-[12px] text-fg"
        />
      ) : (
        <button
          type="button"
          aria-pressed={selected}
          title="Click to select · Shift adds · double click to rename"
          onClick={e => onPick(row.id, e.shiftKey)}
          onDoubleClick={start}
          className="min-w-0 flex-1 truncate text-left font-mono text-[12px] text-fg"
        >
          {row.name}
          {joined ? <span className="ml-1.5 text-fg-mute">+ {joined}</span> : null}
        </button>
      )}
      {row.hidden ? (
        <Icon name="cube-transparent" size={12} className="flex-none text-fg-mute" />
      ) : null}
      {row.joined.length ? (
        <IconButton
          icon="square-split-horizontal"
          size="sm"
          label={`Undo the join into ${row.name}`}
          onClick={() => onUnmerge(row.id)}
        />
      ) : null}
      <span className="flex-none font-mono text-[11.5px] text-fg-mute tabular-nums">
        {formatK(row.triangles)}
      </span>
    </li>
  );
}

/** The parts as they ship, in file order. */
export default function PartsList(props: PartsListProps) {
  const items = props.rows.map((row, i) => (
    <PartItem
      key={row.id}
      row={row}
      index={i}
      selected={props.selected.includes(row.id)}
      onPick={props.onPick}
      onRename={props.onRename}
      onUnmerge={props.onUnmerge}
    />
  ));
  return (
    <ul aria-label="Parts" className="flex flex-col gap-0.5">
      {items}
    </ul>
  );
}

export interface PartActionsProps {
  count: number;
  allHidden: boolean;
  onMerge: () => void;
  onHide: () => void;
}

/** Merge and Hide for the selected parts. */
export function PartActions({ count, allHidden, onMerge, onHide }: PartActionsProps) {
  return (
    <div className="flex items-center gap-2">
      <span className="flex-1 text-meta text-fg-mute">
        {count ? `${count} selected` : 'None selected'}
      </span>
      <Button
        size="sm"
        icon="stack"
        disabled={count < 2}
        disabledReason="Select two or more parts to merge"
        tooltip="Joins the selected parts into one, named after the first. Ships in the next build."
        onClick={onMerge}
      >
        Merge
      </Button>
      <Button
        size="sm"
        icon={allHidden ? 'eye' : 'cube-transparent'}
        disabled={count === 0}
        disabledReason="Select a part first"
        tooltip="Draws the parts as a ghost here. They still ship."
        onClick={onHide}
      >
        {allHidden ? 'Show' : 'Hide'}
      </Button>
    </div>
  );
}
