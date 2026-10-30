import { Button } from '@maquette/ui';
import { Menu } from '@/features/common/Menu';
import { SORT_LABEL, type LibrarySort } from '@/state/library';
import { useSortMenu } from './LibrarySummary.hooks';

export interface LibrarySummaryProps {
  /** "All models", "Models found" or the collection's name. */
  title: string;
  /** "40 models", "12 of 214 models". */
  count: string;
  /** Open look-alike and same name pairs; null before the first scan. */
  pairs: number | null;
  untagged: number;
  sort: LibrarySort;
  onSort: (sort: LibrarySort) => void;
  onLookAlikes: () => void;
  onUntagged: () => void;
}

/** The line over the models: what is shown, the pairs to look at, the untagged ones and the sort. */
export default function LibrarySummary(props: LibrarySummaryProps) {
  const { sorting, toggle, close } = useSortMenu();
  const sorts = (Object.keys(SORT_LABEL) as LibrarySort[]).map(value => ({
    id: value,
    label: SORT_LABEL[value],
    icon: value === props.sort ? ('check' as const) : undefined,
    onSelect: () => props.onSort(value),
  }));
  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
      <h2 className="text-panel-title">{props.title}</h2>
      <span className="text-meta text-fg-mute">{props.count}</span>
      <div className="ml-auto flex flex-wrap items-center justify-end gap-x-3 gap-y-1">
        {props.pairs ? (
          <Button
            size="sm"
            variant="ghost"
            icon="copy"
            tooltip="Models that look the same under two names, or share one name"
            onClick={props.onLookAlikes}
          >
            {props.pairs} look-alike {props.pairs === 1 ? 'group' : 'groups'}
          </Button>
        ) : null}
        {props.untagged ? (
          <Button
            size="sm"
            variant="ghost"
            icon="tag"
            tooltip="Shows the models with no tag yet"
            onClick={props.onUntagged}
          >
            {props.untagged} untagged
          </Button>
        ) : null}
        <div className="relative">
          <Button
            size="sm"
            variant="ghost"
            icon="caret-up-down"
            aria-expanded={sorting}
            aria-label={`Sort: ${SORT_LABEL[props.sort]}`}
            tooltip="Changes the order of the models"
            onClick={toggle}
          >
            {SORT_LABEL[props.sort]}
          </Button>
          {sorting ? (
            <Menu
              label="Sort"
              onClose={close}
              className="absolute top-full right-0 mt-1 w-[200px]"
              items={sorts}
            />
          ) : null}
        </div>
      </div>
    </div>
  );
}
