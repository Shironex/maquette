import { BudgetBar, Button, IconButton, NumberField, Toggle, cn } from '@maquette/ui';
import type { LayoutView } from '@/ipc/client';
import {
  DEFAULT_DESK,
  MAX_STOPS,
  firstViewIds,
  firstViewLine,
  firstViewMb,
  placementOf,
} from '@/state/layout';
import type { Selection } from '@/state/use-layout-store';
import { Hairline } from '../Hairline';
import { SectionTitle } from '../SectionTitle';
import { fixed } from './fixed';
import { useLayoutPanel } from './LayoutPanel.hooks';
import { Selected } from './Selected';
import { sizeLine } from './size-line';

export interface LayoutPanelProps {
  view: LayoutView;
  dirty: boolean;
}

/** The Layout editor's side panel: the selection's fields, the models, the stops, the desk and the first view. */
export default function LayoutPanel({ view, dirty }: LayoutPanelProps) {
  const { store } = useLayoutPanel();
  const { draft, selection } = store;
  const inView = firstViewIds(view);
  const fv = firstViewMb(view.firstView);
  const pick = (next: Selection) => store.select(next);
  const modelRows = view.models.map(m => {
    const on = selection?.kind === 'model' && selection.id === m.asset;
    const p = placementOf(draft, m);
    return (
      <li key={m.asset}>
        <button
          type="button"
          aria-pressed={on}
          aria-label={inView.has(m.asset) ? `${m.name}, in the first view` : m.name}
          onClick={() => pick({ kind: 'model', id: m.asset })}
          className={cn(
            'flex w-full cursor-pointer items-center gap-2 rounded-control px-2 py-1.5 text-left hover:bg-hover',
            on && 'bg-clay-soft hover:bg-clay-soft'
          )}
        >
          <span
            title={inView.has(m.asset) ? 'In the first view' : undefined}
            className={cn(
              'size-[7px] flex-none rounded-full',
              inView.has(m.asset) ? 'bg-sage' : 'bg-line-strong'
            )}
          />
          <span className={cn('min-w-0 flex-1 truncate', on && 'text-clay-text')}>{m.name}</span>
          <span className="flex-none font-mono text-[11.5px] text-fg-mute">
            {sizeLine(m, p.scale).replace(' cm', '')}
          </span>
        </button>
      </li>
    );
  });
  const stopRows = draft.stops.map((s, i) => {
    const on = selection?.kind === 'stop' && selection.id === s.id;
    const looking = store.preview?.id === s.id;
    return (
      <li key={s.id} className="group flex items-center">
        <button
          type="button"
          aria-pressed={on}
          onClick={() => pick({ kind: 'stop', id: s.id })}
          className={cn(
            'flex min-w-0 flex-1 cursor-pointer items-center gap-2 rounded-control px-2 py-1.5 text-left hover:bg-hover',
            on && 'bg-clay-soft hover:bg-clay-soft'
          )}
        >
          <span
            className={cn(
              'grid size-5 flex-none place-items-center rounded-[5px] font-mono text-[11px]',
              i === 0 ? 'bg-sage-soft text-sage' : 'bg-raised text-fg-mute'
            )}
          >
            {i + 1}
          </span>
          <span className={cn('min-w-0 flex-1 truncate', on && 'text-clay-text')}>
            {s.name || 'Unnamed'}
          </span>
          <span className="flex-none font-mono text-[11.5px] text-fg-mute">
            {s.fov.toFixed(0)}°
          </span>
        </button>
        <IconButton
          icon="eye"
          size="sm"
          label={looking ? 'Stop looking' : `Look through ${s.name}`}
          hint={looking ? undefined : 'Shows what a visitor sees from this stop'}
          aria-pressed={looking}
          className={cn('ml-1', looking && 'text-clay-text')}
          onClick={() => store.lookThrough(looking ? null : s.id)}
        />
      </li>
    );
  });
  return (
    <aside
      aria-label="Layout details"
      className="flex w-[340px] flex-none flex-col gap-4 overflow-y-auto border-l border-line bg-panel px-5 py-[18px]"
    >
      <div className="flex flex-col gap-2.5">
        <SectionTitle className="gap-2">
          {selection?.kind === 'stop' ? 'Selected stop' : 'Selected model'}
        </SectionTitle>
        <Selected view={view} layout={draft} selection={selection} />
      </div>
      <Hairline />
      <div className="flex flex-col gap-2">
        <SectionTitle className="gap-2" aside={`${view.models.length}`}>
          Models
        </SectionTitle>
        <ul className="-mx-2 flex flex-col">{modelRows}</ul>
      </div>
      <Hairline />
      <div className="flex flex-col gap-2">
        <SectionTitle className="gap-2" aside={`${draft.stops.length} of ${MAX_STOPS}`}>
          Camera stops
        </SectionTitle>
        {draft.stops.length > 0 ? (
          <ol className="-mx-2 flex flex-col">{stopRows}</ol>
        ) : (
          <p className="text-meta text-fg-mute">
            No stops yet. Frame a view, then add it. The first stop is where the page opens.
          </p>
        )}
        <Button
          size="sm"
          icon="plus"
          className="self-start"
          disabled={!store.pose || draft.stops.length >= MAX_STOPS}
          disabledReason={
            draft.stops.length >= MAX_STOPS
              ? `A scene keeps ${MAX_STOPS} stops at most`
              : 'Open the scene view first'
          }
          tooltip="Adds a camera stop where the view is now"
          onClick={store.addStopHere}
        >
          Add stop from view
        </Button>
      </div>
      <Hairline />
      <div className="flex flex-col gap-2.5">
        <SectionTitle className="gap-2">Desk</SectionTitle>
        <Toggle
          label="A desk under the models"
          description="A plain slab with its top at 0, so the models cast their shadows on it."
          checked={draft.desk !== null}
          onChange={on => store.setDesk(on ? DEFAULT_DESK : null)}
        />
        {draft.desk ? (
          <div className="grid grid-cols-2 gap-2">
            <NumberField
              label="Width"
              size="sm"
              unit="m"
              min={0.1}
              max={50}
              value={draft.desk[0]}
              format={fixed(2)}
              onChange={w => store.setDesk([w, draft.desk?.[1] ?? DEFAULT_DESK[1]])}
            />
            <NumberField
              label="Depth"
              size="sm"
              unit="m"
              min={0.1}
              max={50}
              value={draft.desk[1]}
              format={fixed(2)}
              onChange={d => store.setDesk([draft.desk?.[0] ?? DEFAULT_DESK[0], d])}
            />
          </div>
        ) : null}
      </div>
      <Hairline />
      <div className="flex flex-col gap-2.5">
        <SectionTitle className="gap-2" aside={dirty ? 'Save to measure again' : undefined}>
          First view
        </SectionTitle>
        <BudgetBar
          label={view.firstView.stop ? `From ${view.firstView.stop}` : 'No stop yet'}
          value={fv.value}
          target={fv.target}
          unit="MB"
          format={n => n.toFixed(1)}
        />
        <p className="text-meta text-fg-mute">{firstViewLine(view.firstView)}</p>
      </div>
    </aside>
  );
}
