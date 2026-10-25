import { Button, HelpMark } from '@maquette/ui';
import type { CompressPreset, PipelineView } from '@/ipc/client';
import { formatBytes } from '@/lib/format';
import { PresetTableRow } from './PresetTableRow';
import { usePresetsPanel } from './PresetsPanel.hooks';

export interface PresetsPanelProps {
  view: PipelineView;
  onSave: (presets: CompressPreset[]) => Promise<boolean>;
}

/** Per model texture size, codec and meshopt level, feeding the compress step. */
export default function PresetsPanel({ view, onSave }: PresetsPanelProps) {
  const { rows, open, toggle, dirty, locks, update, save, saving, transfer } = usePresetsPanel(
    view,
    onSave
  );
  const tableRows = rows.map(row => (
    <PresetTableRow
      key={row.model}
      row={row}
      split={open.includes(row.model)}
      onToggle={toggle}
      onUpdate={update}
    />
  ));
  const lockNotes = locks.map(r => (
    <p key={r.model} className="text-meta mt-2 rounded-[8px] bg-raised px-2.5 py-2 text-fg-dim">
      {r.lock}
    </p>
  ));

  return (
    <section
      aria-labelledby="presets-title"
      className="flex w-[380px] flex-none flex-col gap-3 rounded-card border border-line bg-panel px-4 pt-4 pb-3.5"
    >
      <div>
        <div className="flex items-center gap-1.5">
          <h2 id="presets-title" className="text-[15px] leading-5 font-semibold">
            Compression presets
          </h2>
          <HelpMark
            label="About compression presets"
            tip="Each row sets the texture size, codec and meshopt level the Compress step uses for one model. The sliders button next to a name gives its data maps their own size."
            side="bottom"
          />
        </div>
        <p className="text-meta mt-0.5 text-fg-mute">Per model overrides for this project</p>
      </div>
      {/* Scrolls when the log beside it is short; the fade shows there is more. */}
      <div className="min-h-0 flex-1 overflow-auto pb-4 [mask-image:linear-gradient(to_bottom,black_calc(100%-16px),transparent)]">
        <table className="w-full table-fixed border-collapse text-[12px]">
          <colgroup>
            <col />
            <col className="w-[78px]" />
            <col className="w-[84px]" />
            <col className="w-[94px]" />
          </colgroup>
          <thead>
            <tr className="text-left text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase">
              <th className="py-1 font-semibold">Model</th>
              <th className="py-1 font-semibold">Tex</th>
              <th className="py-1 font-semibold">Codec</th>
              <th className="py-1 font-semibold">Meshopt</th>
            </tr>
          </thead>
          <tbody>
            {rows.length === 0 ? (
              <tr>
                <td colSpan={4} className="border-t border-line py-2 text-fg-mute">
                  No models yet. Export runs first, then each GLB gets a row.
                </td>
              </tr>
            ) : null}
            {tableRows}
          </tbody>
        </table>
        {lockNotes}
      </div>
      <div className="flex items-center gap-3 border-t border-line pt-3">
        <div className="flex-1">
          <div className="text-meta text-fg-mute">Est. transfer after Compress</div>
          <div className="font-mono text-[15px] leading-5 tabular-nums">
            {transfer > 0 ? formatBytes(transfer) : 'not built yet'}
          </div>
        </div>
        {dirty ? (
          <span role="status" className="text-meta text-clay-text">
            Not applied yet
          </span>
        ) : null}
        <Button
          variant="primary"
          loading={saving}
          disabled={!dirty}
          disabledReason="Nothing changed yet"
          onClick={() => void save()}
        >
          Apply presets
        </Button>
      </div>
    </section>
  );
}
