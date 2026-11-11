import { Button, InlineBanner, Loader, NumberField, SegmentedControl, Select } from '@maquette/ui';
import { GripGrid } from '@/features/common/GripGrid';
import { GripPicture } from '@/features/common/GripPicture';
import type { GripHands, LibraryGripSummary } from '@/ipc/client';
import { counted } from '@/lib/format';
import { RecipeNote } from '../RecipeNote';
import { useGripAside, useGripPicker } from './GripPicker.hooks';

const whole = (v: number) => String(Math.round(v));
const HANDS: { value: GripHands; label: string }[] = [
  { value: 'both', label: 'Both' },
  { value: 'right', label: 'Right' },
  { value: 'left', label: 'Left' },
];
const CAPTION = 'text-[12px] leading-4 font-medium text-fg-dim';

/** The picked grip: its picture for the hands asked for, its settings and Add grip to recipe. */
function Aside({ grip }: { grip: LibraryGripSummary }) {
  const a = useGripAside(grip);
  return (
    <aside
      aria-label={`Add ${grip.name}`}
      className="flex w-full flex-none flex-col gap-2.5 rounded-card border border-line bg-panel p-3 xl:w-[340px]"
    >
      <div className="flex items-center gap-2">
        <h3 className="min-w-0 flex-1 truncate text-[13px] font-semibold">{grip.name}</h3>
        <span className="rounded-full bg-raised px-2 py-0.5 font-mono text-[11px] text-fg-dim">
          r{grip.latest}
        </span>
      </div>
      <div className="grid h-[150px] place-items-center rounded-card border border-line bg-studio p-3">
        {a.shown ? (
          <GripPicture pose={a.shown.pose} holds={a.shown.holds} label={`${grip.name}, as added`} />
        ) : null}
      </div>
      <div className="flex flex-col gap-1">
        <span aria-hidden className={CAPTION}>
          Hands
        </span>
        <SegmentedControl<GripHands>
          label="Hands"
          tone="clay"
          block
          options={HANDS}
          value={a.hands}
          onChange={a.setHands}
        />
      </div>
      <Select
        label="Holds"
        size="sm"
        options={a.targets}
        value={a.target ?? ''}
        onChange={(v: string) => a.setTarget(v || null)}
      />
      <div className="grid grid-cols-3 gap-2">
        <NumberField
          label="From"
          size="sm"
          unit="f"
          min={0}
          max={a.frames}
          format={whole}
          value={a.from}
          onChange={v => a.setFrom(Math.round(v))}
        />
        <NumberField
          label="To"
          size="sm"
          unit="f"
          min={0}
          max={a.frames}
          format={whole}
          value={a.to}
          onChange={v => a.setTo(Math.round(v))}
        />
        <NumberField
          label="Eases in"
          size="sm"
          unit="f"
          min={0}
          max={600}
          format={whole}
          value={a.easeIn}
          onChange={v => a.setEaseIn(Math.round(v))}
        />
      </div>
      <RecipeNote
        icon="info"
        title={`It poses ${counted(a.fingers, 'finger bone')}`}
        detail="A rig without fingers gets the hand and forearm only, and the review says so."
      />
      {a.older ? (
        <RecipeNote
          icon="warning"
          warn
          title={`${grip.name} has r${grip.latest}`}
          detail={`This recipe holds r${a.older.revision} in ${a.older.layer}. Update it there to take r${grip.latest}, or keep r${a.older.revision}.`}
        />
      ) : null}
      <div className="mt-auto flex justify-end">
        <Button
          size="sm"
          variant="primary"
          icon="plus"
          disabled={Boolean(a.blocked)}
          disabledReason={a.blocked}
          onClick={a.add}
        >
          Add grip to recipe
        </Button>
      </div>
    </aside>
  );
}

/**
 * Add a grip: the grip library as a grid under its weapon filters, and the
 * picked grip beside it with its hands, the blank it holds, where it holds
 * and how it eases in. Adding lays a copy of it on the recipe with a link
 * back to the library grip and its revision.
 */
export default function GripPicker() {
  const v = useGripPicker();
  const body =
    v.list === null ? (
      v.error ? null : (
        <Loader label="Reading the grips..." />
      )
    ) : v.list.length === 0 ? (
      <p className="py-6 text-center text-meta text-fg-mute">
        No grips in the library yet. Save one from pose keys on the hands, with Save a grip.
      </p>
    ) : (
      <GripGrid
        list={v.list}
        filter={v.filter}
        onFilter={v.setFilter}
        selected={v.grip?.id ?? null}
        onSelect={v.select}
        label="Grips to add"
      />
    );
  return (
    <section
      aria-label="Add a grip"
      className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto xl:flex-row"
    >
      <div className="flex min-w-0 flex-1 flex-col gap-3">
        <div className="flex items-center gap-2">
          <h3 className="flex-1 text-[13px] font-semibold">
            Grips in the library{' '}
            {v.list ? <span className="font-normal text-fg-mute">{v.list.length}</span> : null}
          </h3>
          <Button size="sm" variant="ghost" icon="x" onClick={v.close}>
            Close
          </Button>
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
        <p className="text-meta text-fg-mute">
          A recipe keeps its own copy of the grip it uses and a link back, so an exported recipe
          still holds it. A newer revision is offered, never forced.
        </p>
      </div>
      {v.grip ? <Aside key={v.grip.id} grip={v.grip} /> : null}
    </section>
  );
}
