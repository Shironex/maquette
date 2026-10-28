/**
 * Make the other views: Codex draws the views the asset is missing from
 * its front, a few candidates each, with the project's style board when
 * the switch is on. The candidates come back per view; "Use as Left" puts
 * one in its slot.
 */
import { Button, Checkbox, Icon, SegmentedControl, Toggle } from '@maquette/ui';
import type { RefsView } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { inProject } from '@/state/lab';
import { DRAWN, VIEW_LABEL } from '@/state/refs';
import { SectionTitle } from '../../SectionTitle';
import { useOtherViews } from './OtherViews.hooks';

export interface OtherViewsProps {
  view: RefsView;
}

const COUNTS = ['1', '2', '3', '4'].map(n => ({ value: n, label: n }));

export default function OtherViews({ view }: OtherViewsProps) {
  const o = useOtherViews(view);
  const url = (file: string) => fileUrl(inProject(view.projectDir, file)) ?? '';
  const boxes = DRAWN.map(v => (
    <Checkbox
      key={v}
      checked={o.form.views.includes(v)}
      onChange={on => o.toggleView(v, on)}
      label={o.filled.includes(v) ? `${VIEW_LABEL[v]} (filled, draws again)` : VIEW_LABEL[v]}
    />
  ));
  const groups = DRAWN.filter(v => o.byView[v].images.length + o.byView[v].drawing > 0).map(v => {
    const label = VIEW_LABEL[v];
    const picks = o.byView[v].images.map(c => (
      <li key={c.file} className="flex min-w-0 flex-col gap-1">
        <span className="aspect-square overflow-hidden rounded-[8px] border border-line bg-studio">
          <img src={url(c.file)} alt={`${label} candidate`} className="size-full object-cover" />
        </span>
        <Button
          size="sm"
          variant="secondary"
          disabled={o.busy || c.inSlot}
          onClick={() => void o.use(c.file, v)}
        >
          {c.inSlot ? 'In use' : `Use as ${label}`}
        </Button>
      </li>
    ));
    const pending = Array.from({ length: o.byView[v].drawing }, (_, n) => (
      <li
        key={`drawing-${n}`}
        aria-label={`Drawing the ${v}`}
        className="flex aspect-square items-center justify-center rounded-[8px] border border-dashed border-line-strong text-fg-mute"
      >
        <Icon name="circle-notch" spin size={16} />
      </li>
    ));
    return (
      <div key={v} className="flex flex-col gap-1.5">
        <span className="text-meta font-semibold text-fg-dim">{label}</span>
        <ul className="grid list-none grid-cols-4 gap-2 p-0">
          {picks}
          {pending}
        </ul>
      </div>
    );
  });

  return (
    <>
      <div className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-5 py-4">
        <section className="flex items-center gap-3">
          <span className="size-16 flex-none overflow-hidden rounded-[8px] border border-line bg-studio">
            {o.front ? (
              <img src={url(o.front)} alt="The front" className="size-full object-cover" />
            ) : null}
          </span>
          <p className="text-meta text-fg-dim">
            Drawn from the front, <code className="font-mono text-fg">{o.frontName}</code>. Each
            view keeps the object, its proportions and its materials, seen from that side.
          </p>
        </section>

        <section className="flex flex-col gap-2.5">
          <SectionTitle aside="Filled views are skipped unless you tick them">
            Views to draw
          </SectionTitle>
          <div className="flex flex-wrap gap-4">{boxes}</div>
          <div className="flex items-center justify-between gap-3">
            <span className="text-fg-dim">Images per view</span>
            <SegmentedControl
              label="Images per view"
              options={COUNTS}
              value={String(o.form.count)}
              onChange={n => o.setForm({ count: Number(n) })}
            />
          </div>
          <Toggle
            checked={o.form.useStyle}
            onChange={useStyle => o.setForm({ useStyle })}
            label="Use the project's style board"
            description="The views keep the project's look. An empty board carries nothing."
          />
        </section>

        <section className="flex flex-col gap-3">
          <SectionTitle aside={o.drawing > 0 ? `${o.drawing} drawing` : undefined}>
            Candidates
          </SectionTitle>
          {groups}
          {o.total === 0 ? (
            <p className="text-meta text-fg-mute">
              Nothing drawn yet. Candidates land here and in the Lab, under {view.asset.name} views.
            </p>
          ) : null}
        </section>
      </div>
      <footer className="flex items-center justify-end gap-2 border-t border-line px-5 py-3">
        <span className="mr-auto text-meta text-fg-mute">{o.cost}</span>
        <Button variant="secondary" onClick={o.back}>
          Back to the refs
        </Button>
        <Button
          variant="primary"
          icon="sparkle"
          loading={o.busy}
          disabled={o.form.views.length === 0 || o.busy}
          disabledReason={o.form.views.length === 0 ? 'Tick a view to draw' : undefined}
          tooltip="Starts the Codex jobs; each image spends Codex usage, like the Lab"
          onClick={() => void o.start()}
        >
          {o.drawLabel}
        </Button>
      </footer>
    </>
  );
}
