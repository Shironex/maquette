import { Button, HelpMark, Select, cn } from '@maquette/ui';
import type { CSSProperties } from 'react';
import type { CompressPreset, PipelineView, TextureFormat } from '@/ipc/client';
import { formatBytes } from '@/lib/format';
import { PresetTableRow } from './PresetTableRow';
import { SORTS } from './modelResults';
import { usePresetsPanel } from './PresetsPanel.hooks';

export interface PresetsPanelProps {
  view: PipelineView;
  /** How the project's textures ship; WebP and AVIF have no codec choice. */
  textures?: TextureFormat | null;
  onSave: (presets: CompressPreset[]) => Promise<boolean>;
  /** Opens Ship settings at the size report, with each model's bytes and triangles. */
  onSizeReport?: () => void;
  className?: string;
  style?: CSSProperties;
}

const HEAD = 'py-1.5 font-semibold';

/**
 * Every model the pipeline exports: its texture size, codec and meshopt level
 * for the Compress step, and what it shipped at in the last run.
 */
export default function PresetsPanel({
  view,
  textures,
  onSave,
  onSizeReport,
  className,
  style,
}: PresetsPanelProps) {
  const image = textures === 'webp' ? 'WebP' : textures === 'avif' ? 'AVIF' : null;
  const codecOff = image
    ? `The textures ship as ${image}, so the KTX2 codec is not used. Change it in Ship settings`
    : null;
  const p = usePresetsPanel(view, onSave);
  const tableRows = p.rows.map(row => (
    <PresetTableRow
      key={row.model}
      row={row}
      result={p.results.get(row.model)}
      split={p.open.includes(row.model)}
      codecOff={codecOff}
      onToggle={p.toggle}
      onUpdate={p.update}
    />
  ));
  const lockNotes = p.locks.map(r => (
    <p key={r.model} className="text-meta mt-2 rounded-[8px] bg-raised px-2.5 py-2 text-fg-dim">
      {r.lock}
    </p>
  ));

  return (
    <section
      aria-labelledby="presets-title"
      style={style}
      className={cn(
        'flex min-w-0 flex-none flex-col gap-3 rounded-card border border-line bg-panel px-4 pt-3.5 pb-3.5',
        className
      )}
    >
      <div className="flex items-start gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-1.5">
            <h2 id="presets-title" className="text-[15px] leading-5 font-semibold">
              Compression presets
            </h2>
            <HelpMark
              label="About compression presets"
              tip="Each row sets the texture size, codec and meshopt level the Compress step uses for one model, and shows the size it shipped at in the last run. The sliders button next to a name gives its data maps their own size."
              side="bottom"
            />
          </div>
          <p className="text-meta mt-0.5 truncate font-mono text-fg-mute tabular-nums">
            {p.summary}
          </p>
        </div>
        <Select
          aria-label="Sort the models by"
          size="sm"
          options={SORTS}
          value={p.sort}
          onChange={p.setSort}
          className="w-[148px]"
        />
        {onSizeReport ? (
          <Button
            size="sm"
            variant="ghost"
            icon="ruler"
            tooltip="Opens Ship settings at the size report: each model's source and shipped bytes and triangles"
            onClick={onSizeReport}
          >
            Size report
          </Button>
        ) : null}
      </div>
      {/* Scrolls inside; the fade shows there is more. */}
      <div className="-mx-1 min-h-0 flex-1 overflow-auto px-1 pb-4 [mask-image:linear-gradient(to_bottom,black_calc(100%-16px),transparent)]">
        <table className="w-full table-fixed border-collapse text-[12px]">
          <colgroup>
            <col />
            <col className="w-[76px]" />
            <col className="w-[82px]" />
            <col className="w-[100px]" />
            <col className="w-[112px]" />
          </colgroup>
          <thead className="sticky top-0 z-10 bg-panel">
            <tr className="text-left text-[11px] tracking-[0.04em] text-fg-mute uppercase">
              <th className={HEAD}>Model</th>
              <th className={HEAD}>Tex</th>
              <th className={HEAD}>Codec</th>
              <th className={HEAD}>Meshopt</th>
              <th className={cn(HEAD, 'text-right')}>Ships</th>
            </tr>
          </thead>
          <tbody>
            {p.rows.length === 0 ? (
              <tr>
                <td colSpan={5} className="border-t border-line py-2 text-fg-mute">
                  No models yet. Export runs first, then each GLB gets a row.
                </td>
              </tr>
            ) : null}
            {tableRows}
          </tbody>
        </table>
        {lockNotes}
        {image ? (
          <p className="text-meta mt-2 rounded-[8px] bg-raised px-2.5 py-2 text-fg-dim">
            Textures ship as {image}: sizes still apply, the codec column does not.
          </p>
        ) : null}
      </div>
      <div className="flex items-center gap-3 border-t border-line pt-3">
        <div className="flex-1">
          <div className="text-meta text-fg-mute">Est. transfer after Compress</div>
          <div className="font-mono text-[15px] leading-5 tabular-nums">
            {p.transfer > 0 ? formatBytes(p.transfer) : 'not built yet'}
          </div>
        </div>
        {p.dirty ? (
          <span role="status" className="text-meta text-clay-text">
            Not applied yet
          </span>
        ) : null}
        <Button
          variant="primary"
          loading={p.saving}
          disabled={!p.dirty}
          disabledReason="Nothing changed yet"
          tooltip="Saves the presets; Compress uses them on the next run"
          onClick={() => void p.save()}
        >
          Apply presets
        </Button>
      </div>
    </section>
  );
}
