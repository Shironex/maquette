import { Checkbox, cn } from '@maquette/ui';
import type { InboxItem } from '@/ipc/client';
import { formatK } from '@/lib/format';
import { itemTitle } from '@/state/inbox';
import {
  checksCell,
  fromCell,
  groupItems,
  matchedCell,
  triageGroup,
  versionCell,
  type TriageGroup,
} from '@/state/triage';
import { RowThumb } from '../InboxRow/RowThumb';
import { GROUP_ROWS, useTriageTable } from './TriageTable.hooks';

export interface TriageTableProps {
  items: readonly InboxItem[];
  checked: readonly string[];
  onCheck: (id: string, on: boolean) => void;
  onCheckAll: (ids: readonly string[], on: boolean) => void;
}

const TONE: Record<TriageGroup, string> = {
  pass: 'text-sage',
  review: 'text-clay-text',
  replaced: 'text-fg-mute',
  unreadable: 'text-danger',
};

const HEAD =
  'px-2 py-2 whitespace-nowrap text-left text-caption font-normal tracking-[0.04em] text-fg-mute uppercase';
const CELL = 'px-2 py-1.5 align-middle';

function Row({
  item,
  checked,
  onCheck,
  goes,
  onOpen,
}: {
  item: InboxItem;
  checked: boolean;
  onCheck: (on: boolean) => void;
  goes: { asset: string; project: string | null };
  onOpen: () => void;
}) {
  const title = itemTitle(item);
  const group = triageGroup(item);
  return (
    <tr className={cn('border-t border-line', checked ? 'bg-clay-soft/60' : 'hover:bg-hover')}>
      <td className={cn(CELL, 'w-9 pl-3')}>
        <Checkbox
          checked={checked}
          onChange={onCheck}
          label={<span className="sr-only">Select {title}</span>}
        />
      </td>
      <td className={CELL}>
        <span className="flex min-w-0 items-center gap-2.5">
          <RowThumb item={item} size="sm" />
          <span className="min-w-0">
            <button
              type="button"
              onClick={onOpen}
              title="Open it one by one, with its preview and checks"
              className="block max-w-full cursor-pointer truncate rounded-control text-left font-semibold hover:underline focus-visible:outline-2 focus-visible:outline-focus"
            >
              {title}
            </button>
            <span className="block truncate font-mono text-[11px] text-fg-mute">{item.id}</span>
          </span>
        </span>
      </td>
      <td className={CELL}>
        <span className="block truncate">{goes.asset}</span>
        {goes.project ? (
          <span className="block truncate text-[11.5px] text-fg-mute">{goes.project}</span>
        ) : null}
      </td>
      <td className={cn(CELL, 'font-mono text-[12px]')}>{versionCell(item)}</td>
      <td className={cn(CELL, 'whitespace-nowrap')}>{fromCell(item)}</td>
      <td className={cn(CELL, 'text-right font-mono text-[12px] whitespace-nowrap')}>
        {item.report ? formatK(item.report.triangles) : ''}
      </td>
      <td className={cn(CELL, 'max-w-[200px]')}>
        <span className={cn('block truncate', TONE[group])} title={checksCell(item)}>
          {checksCell(item)}
        </span>
      </td>
      <td className={cn(CELL, 'pr-3 whitespace-nowrap text-fg-dim')}>{matchedCell(item)}</td>
    </tr>
  );
}

interface GroupBodyProps extends Omit<TriageTableProps, 'items'> {
  group: ReturnType<typeof groupItems>[number];
  table: ReturnType<typeof useTriageTable>;
}

/** One group: its header row with a box for all of it, its rows and "N more". */
function GroupBody({ group, table, checked, onCheck, onCheckAll }: GroupBodyProps) {
  const ids = group.items.map(i => i.id);
  const all = ids.every(id => checked.includes(id));
  const open = table.isOpen(group.id) || group.items.length <= GROUP_ROWS + 1;
  const shown = open ? group.items : group.items.slice(0, GROUP_ROWS);
  const rest = group.items.length - shown.length;
  const rows = shown.map(item => (
    <Row
      key={item.id}
      item={item}
      checked={checked.includes(item.id)}
      onCheck={on => onCheck(item.id, on)}
      goes={table.goesToCell(item)}
      onOpen={() => table.openOne(item.id)}
    />
  ));
  return (
    <tbody aria-label={group.label}>
      <tr className="border-t border-line bg-raised">
        <td className={cn(CELL, 'pl-3')}>
          <Checkbox
            checked={all}
            onChange={on => onCheckAll(ids, on)}
            label={<span className="sr-only">Select every model in {group.label}</span>}
          />
        </td>
        <th colSpan={7} scope="rowgroup" className={cn(CELL, 'text-left font-semibold')}>
          {group.label} <span className="font-mono text-[12px] text-fg-mute">· {ids.length}</span>
        </th>
      </tr>
      {rows}
      {rest > 0 ? (
        <tr className="border-t border-line">
          <td />
          <td colSpan={7} className={CELL}>
            <button
              type="button"
              onClick={() => table.open(group.id)}
              className="cursor-pointer text-clay-text hover:underline"
            >
              {rest} more in {group.label.toLowerCase()}
              {all ? ', all selected' : ''}
            </button>
          </td>
        </tr>
      ) : null}
    </tbody>
  );
}

/** The inbox as one table, grouped by what each model needs, worst last, with a box per row and group. */
export default function TriageTable({ items, checked, onCheck, onCheckAll }: TriageTableProps) {
  const table = useTriageTable();
  const groups = groupItems(items).map(g => (
    <GroupBody
      key={g.id}
      group={g}
      table={table}
      checked={checked}
      onCheck={onCheck}
      onCheckAll={onCheckAll}
    />
  ));
  return (
    <div className="overflow-hidden rounded-card border border-line bg-panel">
      <table className="w-full table-fixed border-collapse">
        <colgroup>
          <col className="w-9" />
          <col className="w-[26%]" />
          <col className="w-[18%]" />
          <col className="w-12" />
          <col className="w-[11%]" />
          <col className="w-16" />
          <col />
          <col className="w-[12%]" />
        </colgroup>
        <thead>
          <tr>
            <th className={cn(HEAD, 'pl-3')}>
              <span className="sr-only">Selected</span>
            </th>
            <th className={HEAD}>Model</th>
            <th className={HEAD}>Goes to</th>
            <th className={HEAD}>Ver</th>
            <th className={HEAD}>From</th>
            <th className={cn(HEAD, 'text-right')}>Tris</th>
            <th className={HEAD}>Checks</th>
            <th className={cn(HEAD, 'pr-3')}>Matched by</th>
          </tr>
        </thead>
        {groups}
      </table>
    </div>
  );
}
