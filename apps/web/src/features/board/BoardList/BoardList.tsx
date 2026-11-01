import { Button, Icon, Tooltip, cn } from '@maquette/ui';
import { useNow } from '@/lib/useNow';
import { useBoardList } from './BoardList.hooks';
import type { BoardListProps } from './BoardList.types';
import { LIST_COLUMNS } from './columns';
import { ListRow } from './ListRow';
import { ListToolbar } from './ListToolbar';

const HEAD = 'sticky top-0 z-10 h-8 border-b border-line bg-panel px-2 text-left font-normal';

/** The project's assets as one dense, sortable table, with the Board's selection and menus. */
export default function BoardList(props: BoardListProps) {
  const { rows, total, prefs, board } = props;
  const list = useBoardList(props);
  const nowMs = useNow(30_000);
  const { sort } = prefs;
  const headings = LIST_COLUMNS.map(col => {
    const on = sort.key === col.key;
    const order = sort.ascending ? 'ascending' : 'descending';
    return (
      <th
        key={col.key}
        scope="col"
        aria-sort={on ? order : undefined}
        className={cn(HEAD, col.width, col.narrow, col.key === 'status' && 'pl-4')}
      >
        <Tooltip label={col.hint} wrap disabled={!col.hint}>
          <button
            type="button"
            onClick={() => list.sortBy(col.key)}
            className={cn(
              'inline-flex h-6 cursor-pointer items-center gap-1 rounded-control text-caption tracking-[0.06em] whitespace-nowrap uppercase hover:text-fg',
              on ? 'text-fg' : 'text-fg-mute',
              col.numeric && 'ml-auto flex flex-row-reverse'
            )}
          >
            {col.label}
            <Icon
              name={on ? (sort.ascending ? 'arrow-up' : 'arrow-down') : 'caret-up-down'}
              size={11}
              className={on ? 'text-clay-text' : 'opacity-0 group-hover/head:opacity-100'}
            />
          </button>
        </Tooltip>
      </th>
    );
  });
  const { shown } = list;
  const spacer = (height: number) =>
    height > 0 ? (
      <tr aria-hidden="true" style={{ height }}>
        <td colSpan={LIST_COLUMNS.length + 3} />
      </tr>
    ) : null;
  const body = rows
    .slice(shown.start, shown.end)
    .map((row, n) => (
      <ListRow
        key={row.asset.id}
        row={row}
        board={board}
        tabStop={list.active === row.asset.id}
        menuUp={list.menuUp}
        onToggleMenu={list.toggleMenu}
        onFocus={list.setFocused}
        nowMs={nowMs}
        position={{ ...shown.itemProps(shown.start + n), 'aria-rowindex': shown.start + n + 2 }}
      />
    ));

  return (
    <div className="absolute inset-0 flex flex-col gap-3">
      <ListToolbar prefs={prefs} shown={rows.length} total={total} onChange={list.change} />
      <div
        ref={list.tableRef}
        className={cn(
          'min-h-0 flex-1 overflow-y-auto rounded-card border border-line bg-panel',
          list.selecting && 'pb-16'
        )}
      >
        <table
          role="grid"
          aria-label="Assets"
          aria-multiselectable="true"
          aria-rowcount={rows.length + 1}
          className="group/head w-full table-fixed border-collapse"
        >
          <thead>
            <tr aria-rowindex={1}>
              <th scope="col" className={cn(HEAD, 'w-[36px] pl-3')}>
                <span className="sr-only">Selected</span>
              </th>
              <th scope="col" className={cn(HEAD, 'w-[60px]')}>
                <span className="sr-only">Picture</span>
              </th>
              {headings}
              <th scope="col" className={cn(HEAD, 'w-[44px]')}>
                <span className="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody ref={list.bodyRef} onKeyDown={list.onKeyDown}>
            {spacer(shown.before)}
            {body}
            {spacer(shown.after)}
          </tbody>
        </table>
        {rows.length === 0 && total > 0 ? (
          <div className="flex flex-col items-center gap-2 py-12 text-meta text-fg-mute">
            No asset matches this stage and search.
            <Button
              size="sm"
              tooltip="Clears the stage filter and the search"
              onClick={list.clearFilters}
            >
              Show every asset
            </Button>
          </div>
        ) : null}
      </div>
    </div>
  );
}
