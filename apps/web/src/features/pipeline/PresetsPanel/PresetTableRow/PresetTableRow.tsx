import { Icon, Select, cn } from '@maquette/ui';
import type { PresetRow } from '@/state/presets';
import { CODECS, DATA_SIZES, MESHOPT, SAME, SIZES } from '../preset-options';

export interface PresetTableRowProps {
  row: PresetRow;
  /** The extra line with the data map size is open. */
  split: boolean;
  onToggle: (model: string) => void;
  onUpdate: (model: string, patch: Partial<PresetRow>) => void;
}

/** One model's texture size, codec and meshopt level, with its data map line. */
export default function PresetTableRow({ row, split, onToggle, onUpdate }: PresetTableRowProps) {
  const locked = Boolean(row.lock);
  return (
    <>
      <tr className="border-t border-line">
        <td className="py-1 pr-1">
          <span className="flex items-center gap-1.5">
            <span className="truncate">{row.label}</span>
            <button
              type="button"
              aria-label={`Texture sizes per map for ${row.label}`}
              aria-expanded={split}
              disabled={locked}
              onClick={() => onToggle(row.model)}
              className={cn(
                'grid size-5 flex-none cursor-pointer place-items-center rounded-control text-fg-mute hover:text-fg disabled:cursor-default disabled:opacity-45',
                split && 'bg-raised text-fg'
              )}
            >
              <Icon name="sliders-horizontal" size={12} />
            </button>
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
          <Select
            aria-label={`${row.label} codec`}
            size="sm"
            options={CODECS}
            value={row.codec}
            disabled={locked}
            onChange={v => onUpdate(row.model, { codec: v })}
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
            onChange={v => onUpdate(row.model, { meshopt: v })}
          />
        </td>
      </tr>
      {split ? (
        <tr>
          <td colSpan={4} className="pb-2">
            <div className="flex items-center justify-between gap-3 rounded-[8px] bg-raised px-2.5 py-2">
              <span className="text-meta text-fg-dim">
                Metal, roughness, occlusion and normal maps
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
              />
            </div>
          </td>
        </tr>
      ) : null}
    </>
  );
}
