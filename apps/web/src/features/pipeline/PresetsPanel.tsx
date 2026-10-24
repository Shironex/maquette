import { useEffect, useMemo, useState } from 'react';
import { Button, Icon, Select, cn } from '@maquette/ui';
import type { CompressPreset, MeshoptLevel, PipelineView, TextureCodec } from '@/ipc/client';
import { formatBytes } from '@/lib/format';
import { estimatedTransfer } from '@/state/pipeline';
import { presetRows, presetsToSave, type PresetRow } from '@/state/presets';

const SIZES = ['512', '1024', '2048', '4096'] as const;
const CODECS: readonly { value: TextureCodec; label: string }[] = [
  { value: 'etc1s', label: 'ETC1S' },
  { value: 'uastc', label: 'UASTC' },
];
const MESHOPT: readonly MeshoptLevel[] = ['low', 'medium', 'high'];

/** Per model texture size, codec and meshopt level, feeding the compress step. */
export function PresetsPanel({
  view,
  onSave,
}: {
  view: PipelineView;
  onSave: (presets: CompressPreset[]) => Promise<boolean>;
}) {
  const initial = useMemo(() => presetRows(view), [view]);
  const [rows, setRows] = useState<PresetRow[]>(initial);
  const [saving, setSaving] = useState(false);
  useEffect(() => setRows(initial), [initial]);
  const dirty = JSON.stringify(rows) !== JSON.stringify(initial);
  const locks = rows.filter(r => r.lock);
  const update = (model: string, patch: Partial<PresetRow>) =>
    setRows(list => list.map(r => (r.model === model ? { ...r, ...patch, custom: true } : r)));
  const save = async () => {
    setSaving(true);
    await onSave(presetsToSave(rows));
    setSaving(false);
  };
  const transfer = estimatedTransfer(view);

  return (
    <section
      aria-labelledby="presets-title"
      className="flex w-[380px] flex-none flex-col gap-3 rounded-card border border-line bg-panel px-4 pt-4 pb-3.5"
    >
      <div>
        <h2 id="presets-title" className="text-[15px] leading-5 font-semibold">
          Compression presets
        </h2>
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
            {rows.map(row => {
              const locked = Boolean(row.lock);
              return (
                <tr key={row.model} className="border-t border-line">
                  <td className="py-1 pr-1">
                    <span className="flex items-center gap-1.5">
                      <span className="truncate">{row.label}</span>
                      {locked ? (
                        <Icon
                          name="lock-simple"
                          size={12}
                          className="text-clay-text"
                          title="Locked"
                        />
                      ) : null}
                    </span>
                  </td>
                  <td className="py-1 pr-1">
                    <Select
                      aria-label={`${row.label} texture size`}
                      size="sm"
                      options={SIZES}
                      value={String(row.textureSize) as (typeof SIZES)[number]}
                      disabled={locked}
                      onChange={v => update(row.model, { textureSize: Number(v) })}
                    />
                  </td>
                  <td className="py-1 pr-1">
                    <Select
                      aria-label={`${row.label} codec`}
                      size="sm"
                      options={CODECS}
                      value={row.codec}
                      disabled={locked}
                      onChange={v => update(row.model, { codec: v })}
                    />
                  </td>
                  <td className="py-1">
                    <Select
                      aria-label={`${row.label} meshopt level`}
                      size="sm"
                      options={MESHOPT}
                      value={row.meshopt}
                      disabled={locked}
                      className={cn(locked && 'text-clay-text')}
                      onChange={v => update(row.model, { meshopt: v })}
                    />
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
        {locks.map(r => (
          <p
            key={r.model}
            className="text-meta mt-2 rounded-[8px] bg-raised px-2.5 py-2 text-fg-dim"
          >
            {r.lock}
          </p>
        ))}
      </div>
      <div className="flex items-center gap-3 border-t border-line pt-3">
        <div className="flex-1">
          <div className="text-meta text-fg-mute">Est. transfer after Compress</div>
          <div className="font-mono text-[15px] leading-5 tabular-nums">
            {transfer > 0 ? formatBytes(transfer) : 'not built yet'}
          </div>
        </div>
        <Button variant="primary" disabled={!dirty || saving} onClick={() => void save()}>
          Apply presets
        </Button>
      </div>
    </section>
  );
}
