import { SegmentedControl, Select, Toggle } from '@maquette/ui';
import type { RenderLook } from '@/ipc/client';
import { LOOK_OPTIONS } from '@/state/renders';
import { useContactPreview } from './ContactPreview.hooks';

const COLUMNS = [
  { value: '2', label: '2' },
  { value: '3', label: '3' },
  { value: '4', label: '4' },
] as const;

/** The contact sheet of the selection and the folders the render writes. */
export default function ContactPreview() {
  const c = useContactPreview();
  const tiles = c.tiles.map(t => (
    <figure key={t.key} className="relative aspect-square bg-raised">
      {t.url ? <img src={t.url} alt="" className="size-full object-cover" /> : null}
      {c.settings.names ? (
        <figcaption className="absolute bottom-1 left-1 max-w-[90%] truncate rounded-[3px] bg-[rgba(16,15,14,0.72)] px-1 font-mono text-[10px] text-[#f5f1ea]">
          {t.name}
        </figcaption>
      ) : null}
    </figure>
  ));
  const folders = c.folders.map(f => (
    <li key={f.key} className="truncate pl-3">
      {f.line}
    </li>
  ));
  return (
    <>
      <section aria-label="Contact sheet" className="flex flex-col gap-2">
        <div className="flex items-baseline justify-between">
          <h3 className="text-panel-title">Contact sheet</h3>
          <span className="text-meta text-fg-mute">{c.look}, three quarter</span>
        </div>
        {tiles.length > 0 ? (
          <div
            className="grid gap-px overflow-hidden rounded-[10px] border border-line bg-line"
            style={{ gridTemplateColumns: `repeat(${c.settings.columns}, minmax(0, 1fr))` }}
          >
            {tiles}
          </div>
        ) : null}
        <Toggle
          label="Make a contact sheet"
          checked={c.sheet}
          onChange={c.setSheet}
          disabled={c.locked}
        />
        <Toggle
          label="Names under each"
          checked={c.settings.names}
          onChange={names => c.change({ names })}
          disabled={c.locked}
        />
        {!c.locked ? (
          <Select<RenderLook>
            label="Sheet in"
            value={c.settings.look}
            options={LOOK_OPTIONS}
            onChange={look => c.change({ look })}
          />
        ) : null}
        <SegmentedControl
          label="Columns"
          options={COLUMNS}
          value={String(c.settings.columns) as '2' | '3' | '4'}
          onChange={v => c.change({ columns: Number(v) })}
        />
      </section>
      <section aria-label="Saved to the project exports" className="border-t border-line pt-3">
        <h4 className="mb-1 text-meta font-medium tracking-wide text-fg-mute uppercase">
          Saved to the exports
        </h4>
        <ul className="font-mono text-meta text-fg-dim">
          <li>{c.root}</li>
          {folders}
        </ul>
      </section>
    </>
  );
}
