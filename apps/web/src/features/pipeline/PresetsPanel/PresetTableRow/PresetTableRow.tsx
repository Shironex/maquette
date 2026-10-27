import { Icon, Select, Tooltip, cn } from '@maquette/ui';
import { formatBytes } from '@/lib/format';
import { splitSize, type PresetRow } from '@/state/presets';
import { resultText, type ModelResult } from '../modelResults';
import { CODECS, DATA_SIZES, MESHOPT, SAME, SIZES } from '../preset-options';

export interface PresetTableRowProps {
  row: PresetRow;
  /** What the model weighed raw and shipped at in the last run. */
  result?: ModelResult;
  /** The extra line with the data map size is open. */
  split: boolean;
  /** Why the codec cannot be picked, when the textures are not KTX2. */
  codecOff?: string | null;
  onToggle: (model: string) => void;
  onUpdate: (model: string, patch: Partial<PresetRow>) => void;
}

function ResultCell({ result }: { result?: ModelResult }) {
  const text = resultText(result);
  const detail = result?.raw
    ? result.shipped
      ? `${formatBytes(result.raw)} raw export, ${formatBytes(result.shipped)} after Compress`
      : `${formatBytes(result.raw)} raw export; Compress has not run on it yet`
    : 'Export GLB has not written it yet';
  return (
    <Tooltip label="Last run" detail={detail}>
      <span
        tabIndex={0}
        className={cn(
          'block truncate rounded-control text-right font-mono text-[11.5px] tabular-nums',
          result?.shipped ? 'text-fg' : 'text-fg-mute'
        )}
      >
        {result?.raw && result.shipped ? (
          <span className="text-fg-mute">{formatBytes(result.raw)} → </span>
        ) : null}
        {text}
      </span>
    </Tooltip>
  );
}

/** One model's texture size, codec and meshopt level, its result, and its data map line. */
export default function PresetTableRow({
  row,
  result,
  split,
  codecOff,
  onToggle,
  onUpdate,
}: PresetTableRowProps) {
  const locked = Boolean(row.lock);
  const data = splitSize(row);
  return (
    <>
      <tr className={cn('border-t border-line', split && 'bg-raised/40')}>
        <td className="py-1 pr-2">
          <span className="flex min-w-0 items-center gap-1.5">
            <span className="truncate" title={row.label}>
              {row.label}
            </span>
            {data !== null && !split ? (
              <span className="flex-none rounded-control bg-raised px-1.5 font-mono text-[10.5px] leading-[18px] text-fg-dim">
                data {data}
              </span>
            ) : null}
            <Tooltip
              label="Texture sizes per map"
              detail={
                locked
                  ? 'Locked; the note below says why'
                  : 'Gives the data maps (metal, roughness, occlusion, normal) a size of their own'
              }
            >
              <button
                type="button"
                aria-label={`Texture sizes per map for ${row.label}`}
                aria-expanded={split}
                aria-disabled={locked || undefined}
                onClick={() => {
                  if (!locked) onToggle(row.model);
                }}
                className={cn(
                  'ml-auto grid size-5 flex-none cursor-pointer place-items-center rounded-control text-fg-mute hover:not-aria-disabled:text-fg active:not-aria-disabled:bg-line aria-disabled:cursor-default aria-disabled:opacity-45',
                  split && 'bg-raised text-fg'
                )}
              >
                <Icon name="sliders-horizontal" size={12} />
              </button>
            </Tooltip>
            {locked ? (
              <Icon name="lock-simple" size={12} className="text-clay-text" title="Locked" />
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
            onChange={v => onUpdate(row.model, { textureSize: Number(v) })}
          />
        </td>
        <td className="py-1 pr-1">
          <Tooltip label="Codec" detail={codecOff ?? undefined} disabled={!codecOff}>
            <Select
              aria-label={`${row.label} codec`}
              size="sm"
              options={CODECS}
              value={row.codec}
              disabled={locked || Boolean(codecOff)}
              onChange={v => onUpdate(row.model, { codec: v })}
            />
          </Tooltip>
        </td>
        <td className="py-1 pr-2">
          <Select
            aria-label={`${row.label} meshopt level`}
            size="sm"
            options={MESHOPT}
            value={row.meshopt}
            disabled={locked}
            className={cn(locked && 'text-clay-text')}
            onChange={v => onUpdate(row.model, { meshopt: v })}
          />
        </td>
        <td className="py-1">
          <ResultCell result={result} />
        </td>
      </tr>
      {split ? (
        <tr className="bg-raised/40">
          <td colSpan={5} className="pb-2">
            <div className="ml-2 flex items-center gap-3 border-l-2 border-line-strong py-1 pl-3">
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="text-[12px] text-fg">
                  Data maps
                  <span className="text-fg-mute"> · base color stays {row.textureSize} px</span>
                </span>
                <span className="text-meta truncate text-fg-mute">
                  Metal, roughness, occlusion and normal
                </span>
              </span>
              <Select
                aria-label={`${row.label} data map size`}
                size="sm"
                options={DATA_SIZES}
                value={row.dataTextureSize === null ? SAME : String(row.dataTextureSize)}
                disabled={locked}
                onChange={v =>
                  onUpdate(row.model, { dataTextureSize: v === SAME ? null : Number(v) })
                }
                className="w-[132px]"
              />
            </div>
          </td>
        </tr>
      ) : null}
    </>
  );
}
