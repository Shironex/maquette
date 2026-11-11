/**
 * The Animations library's Grips: hand poses kept once outside any
 * project, each with its revisions, saved from a recipe's pose keys. A
 * recipe takes one from its editor's grip picker as a copy with a link
 * back. The selected grip opens beside the grid.
 */
import { Button, EmptyState, InlineBanner, Loader } from '@maquette/ui';
import { GripGrid } from '@/features/common/GripGrid';
import { counted } from '@/lib/format';
import { GripDetail } from '../GripDetail';
import { useLibraryGrips } from './LibraryGrips.hooks';

export default function LibraryGrips() {
  const v = useLibraryGrips();
  const body =
    v.list === null ? (
      v.error ? null : (
        <div className="grid flex-1 place-items-center">
          <Loader label="Reading the grips..." />
        </div>
      )
    ) : v.list.length === 0 ? (
      <EmptyState
        icon="hand-pointing"
        title="No grips yet"
        body="A grip is a hand pose kept for every project. Save one from a recipe's pose keys on the hands, with Save a grip in the pose keys settings."
      />
    ) : (
      <GripGrid
        list={v.list}
        filter={v.filter}
        onFilter={v.setFilter}
        selected={v.summary?.id ?? null}
        onSelect={v.select}
        label="Library grips"
      />
    );
  return (
    <main className="flex min-h-0 flex-1" aria-busy={v.list === null && !v.error}>
      <section
        aria-label="Grips"
        className="flex min-w-0 flex-1 flex-col gap-4 overflow-y-auto px-6 pt-[18px] pb-5"
      >
        <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
          <h2 className="text-panel-title whitespace-nowrap">Grips in the library</h2>
          {v.list ? (
            <span className="text-meta text-fg-mute">{counted(v.list.length, 'grip')}</span>
          ) : null}
        </div>
        {v.error ? (
          <InlineBanner
            tone="danger"
            title="The grip library could not be read"
            detail={v.error}
            actions={
              <Button size="sm" icon="arrow-clockwise" onClick={v.retry}>
                Try again
              </Button>
            }
          />
        ) : null}
        {body}
        {v.list?.length ? (
          <p className="text-meta text-fg-mute">
            Grips live next to recipes in the library, one folder per grip with every revision. A
            recipe keeps its own copy of the grip it uses and a link back, so an exported recipe
            still holds it.
          </p>
        ) : null}
      </section>
      {v.summary ? <GripDetail key={v.summary.id} summary={v.summary} /> : null}
    </main>
  );
}
