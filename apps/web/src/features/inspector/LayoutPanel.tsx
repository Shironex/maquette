import { useId } from 'react';
import {
  BudgetBar,
  Button,
  Checkbox,
  Icon,
  IconButton,
  NumberField,
  TextField,
  Toggle,
  VersionBadge,
  cn,
} from '@maquette/ui';
import type { LayoutModel, LayoutView, Measured } from '@/ipc/client';
import { formatBytes } from '@/lib/format';
import { num, type Vec3 } from '@/state/inspector';
import {
  DEFAULT_DESK,
  MAX_SCALE,
  MAX_STOPS,
  MIN_SCALE,
  dropToFloor,
  firstViewIds,
  firstViewLine,
  firstViewMb,
  placementOf,
  sizeCm,
  type PlainLayout,
  type PlainPlacement,
  type PlainStop,
} from '@/state/layout';
import { useLayoutStore, type Selection } from '@/state/layout-store';

function SectionTitle({ children, aside }: { children: string; aside?: React.ReactNode }) {
  return (
    <div className="flex items-baseline justify-between gap-2">
      <h2 className="text-[11px] leading-[14px] font-medium tracking-[0.04em] text-fg-mute uppercase">
        {children}
      </h2>
      {aside ? <span className="text-meta text-fg-mute">{aside}</span> : null}
    </div>
  );
}

const hair = <div className="h-px flex-none bg-line" />;

const MEASURED: Record<Measured, string> = {
  shipped: 'shipped build',
  raw: 'raw export',
  source: 'source model',
};

const fixed = (digits: number) => (n: number) => n.toFixed(digits);
const AXES = ['X', 'Y', 'Z'] as const;

function Vec3Fields({
  label,
  value,
  unit,
  digits,
  onChange,
}: {
  label: string;
  value: Vec3;
  unit: string;
  digits: number;
  onChange: (value: Vec3) => void;
}) {
  const id = useId();
  return (
    <div role="group" aria-label={`${label} in ${unit}`} className="flex flex-col gap-1.5">
      <span aria-hidden="true" className="text-[12px] leading-4 font-medium text-fg-dim">
        {label} <span className="font-normal text-fg-mute">{unit}</span>
      </span>
      <div className="grid min-w-0 grid-cols-3 gap-1.5">
        {AXES.map((axis, i) => (
          <div key={axis} className="flex min-w-0 items-center gap-1.5">
            <label htmlFor={`${id}-${axis}`} className="w-2 flex-none text-[11px] text-fg-mute">
              {axis}
            </label>
            <NumberField
              id={`${id}-${axis}`}
              size="sm"
              value={value[i]!}
              format={fixed(digits)}
              fieldClassName="min-w-0 flex-1"
              onChange={v => {
                const next = [...value] as Vec3;
                next[i] = v;
                onChange(next);
              }}
            />
          </div>
        ))}
      </div>
    </div>
  );
}

function sizeLine(model: LayoutModel, scale: number): string {
  if (!model.min || !model.max) return 'Size unknown';
  return sizeCm(model, scale)
    .map(v => (v >= 100 ? v.toFixed(0) : v.toFixed(1)))
    .join(' × ')
    .concat(' cm');
}

function ModelDetails({ model, layout }: { model: LayoutModel; layout: PlainLayout }) {
  const store = useLayoutStore();
  const p = placementOf(layout, model);
  const place = (next: Partial<PlainPlacement>) => store.placeModel(model.asset, { ...p, ...next });
  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-start gap-3">
        <span className="grid size-10 flex-none place-items-center rounded-[8px] bg-studio text-fg-mute">
          <Icon name="cube" />
        </span>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span className="truncate text-[15px] leading-5 font-semibold tracking-[-0.01em]">
              {model.name}
            </span>
            <VersionBadge version={model.version} />
          </div>
          <div className="truncate font-mono text-[12px] text-fg-mute">
            {sizeLine(model, p.scale)} · {formatBytes(num(model.bytes))}
          </div>
        </div>
      </div>
      <Vec3Fields
        label="Position"
        unit="m"
        digits={3}
        value={p.position}
        onChange={position => place({ position })}
      />
      <Vec3Fields
        label="Rotation"
        unit="degrees"
        digits={1}
        value={p.rotation}
        onChange={rotation => place({ rotation })}
      />
      <NumberField
        label="Scale"
        size="sm"
        unit="×"
        min={MIN_SCALE}
        max={MAX_SCALE}
        value={p.scale}
        format={fixed(3)}
        onChange={scale => place({ scale })}
        hint={`Uniform. Measured from the ${MEASURED[model.measured]}.`}
      />
      <div className="flex flex-wrap gap-2">
        <Button
          size="sm"
          icon="arrow-down"
          onClick={() => store.placeModel(model.asset, dropToFloor(model, p))}
          title="Moves the model so its lowest point rests on the desk"
        >
          Drop to desk
        </Button>
        <Button
          size="sm"
          icon="arrow-counter-clockwise"
          onClick={() => place({ rotation: [0, 0, 0], scale: 1 })}
        >
          Reset turn and scale
        </Button>
      </div>
    </div>
  );
}

function StopDetails({
  stop,
  index,
  count,
  models,
}: {
  stop: PlainStop;
  index: number;
  count: number;
  models: LayoutModel[];
}) {
  const store = useLayoutStore();
  const looking = store.preview?.id === stop.id;
  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-end gap-2">
        <TextField
          label={index === 0 ? 'Name · the view the page opens on' : 'Name'}
          value={stop.name}
          maxLength={60}
          fieldClassName="flex-1"
          onChange={e => store.setStop(stop.id, { name: e.target.value })}
        />
        <IconButton
          icon="arrow-up"
          label="Earlier in the tour"
          variant="secondary"
          disabled={index === 0}
          onClick={() => store.moveStop(stop.id, -1)}
        />
        <IconButton
          icon="arrow-down"
          label="Later in the tour"
          variant="secondary"
          disabled={index === count - 1}
          onClick={() => store.moveStop(stop.id, 1)}
        />
      </div>
      <div className="flex flex-wrap gap-2">
        <Button
          size="sm"
          icon="video-camera"
          disabled={!store.pose}
          onClick={() => store.setStopFromView(stop.id)}
          title="Puts the stop where the view's camera stands now"
        >
          Set from view
        </Button>
        <Button
          size="sm"
          icon="eye"
          aria-pressed={looking}
          className={cn(looking && 'border-clay-line text-clay-text')}
          onClick={() => store.lookThrough(looking ? null : stop.id)}
        >
          {looking ? 'Stop looking' : 'Look through'}
        </Button>
        <IconButton
          icon="trash"
          label={`Delete ${stop.name || 'this stop'}`}
          size="sm"
          variant="secondary"
          className="ml-auto"
          onClick={() => store.removeStop(stop.id)}
        />
      </div>
      <Vec3Fields
        label="Camera"
        unit="m"
        digits={3}
        value={stop.position}
        onChange={position => store.setStop(stop.id, { position })}
      />
      <Vec3Fields
        label="Looks at"
        unit="m"
        digits={3}
        value={stop.target}
        onChange={target => store.setStop(stop.id, { target })}
      />
      <NumberField
        label="Field of view"
        size="sm"
        unit="degrees"
        min={10}
        max={120}
        value={stop.fov}
        format={fixed(1)}
        onChange={fov => store.setStop(stop.id, { fov })}
        hint="Vertical, at 16:9 like the posters."
      />
      {index === 0 ? (
        <div className="flex flex-col gap-2">
          <Toggle
            label="Count what the camera sees"
            description="The first view budget counts the models in this stop's frame. Turn it off to pick them yourself."
            checked={stop.models === null}
            onChange={on =>
              store.setStop(stop.id, {
                models: on ? null : [...(store.view?.firstView.models.map(m => m.asset) ?? [])],
              })
            }
          />
          {stop.models !== null ? (
            <div className="flex flex-col gap-1.5 pl-1">
              {models.map(m => (
                <Checkbox
                  key={m.asset}
                  label={m.name}
                  checked={stop.models?.includes(m.asset) ?? false}
                  onChange={on =>
                    store.setStop(stop.id, {
                      models: on
                        ? [...(stop.models ?? []), m.asset]
                        : (stop.models ?? []).filter(id => id !== m.asset),
                    })
                  }
                />
              ))}
            </div>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

function Selected({
  view,
  layout,
  selection,
}: {
  view: LayoutView;
  layout: PlainLayout;
  selection: Selection;
}) {
  if (selection?.kind === 'model') {
    const model = view.models.find(m => m.asset === selection.id);
    if (model) return <ModelDetails model={model} layout={layout} />;
  }
  if (selection?.kind === 'stop') {
    const index = layout.stops.findIndex(s => s.id === selection.id);
    const stop = layout.stops[index];
    if (stop) {
      return (
        <StopDetails stop={stop} index={index} count={layout.stops.length} models={view.models} />
      );
    }
  }
  return (
    <p className="text-meta text-fg-mute">
      Click a model or a camera in the view, or pick one below. W, E and R switch between move, turn
      and scale.
    </p>
  );
}

export function LayoutPanel({ view, dirty }: { view: LayoutView; dirty: boolean }) {
  const store = useLayoutStore();
  const { draft, selection } = store;
  const inView = firstViewIds(view);
  const fv = firstViewMb(view.firstView);
  const pick = (next: Selection) => store.select(next);
  return (
    <aside
      aria-label="Layout details"
      className="flex w-[340px] flex-none flex-col gap-4 overflow-y-auto border-l border-line bg-panel px-5 py-[18px]"
    >
      <div className="flex flex-col gap-2.5">
        <SectionTitle>
          {selection?.kind === 'stop' ? 'Selected stop' : 'Selected model'}
        </SectionTitle>
        <Selected view={view} layout={draft} selection={selection} />
      </div>
      {hair}
      <div className="flex flex-col gap-2">
        <SectionTitle aside={`${view.models.length}`}>Models</SectionTitle>
        <ul className="-mx-2 flex flex-col">
          {view.models.map(m => {
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
                  <span className={cn('min-w-0 flex-1 truncate', on && 'text-clay-text')}>
                    {m.name}
                  </span>
                  <span className="flex-none font-mono text-[11.5px] text-fg-mute">
                    {sizeLine(m, p.scale).replace(' cm', '')}
                  </span>
                </button>
              </li>
            );
          })}
        </ul>
      </div>
      {hair}
      <div className="flex flex-col gap-2">
        <SectionTitle aside={`${draft.stops.length} of ${MAX_STOPS}`}>Camera stops</SectionTitle>
        {draft.stops.length > 0 ? (
          <ol className="-mx-2 flex flex-col">
            {draft.stops.map((s, i) => {
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
                    aria-pressed={looking}
                    className={cn('ml-1', looking && 'text-clay-text')}
                    onClick={() => store.lookThrough(looking ? null : s.id)}
                  />
                </li>
              );
            })}
          </ol>
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
          onClick={store.addStopHere}
        >
          Add stop from view
        </Button>
      </div>
      {hair}
      <div className="flex flex-col gap-2.5">
        <SectionTitle>Desk</SectionTitle>
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
      {hair}
      <div className="flex flex-col gap-2.5">
        <SectionTitle aside={dirty ? 'Save to measure again' : undefined}>First view</SectionTitle>
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
