import { SegmentedControl, cn, type SegmentedOption } from '@maquette/ui';
import type { InspectModel, TextureCodec } from '@/ipc/client';
import { MAP_GROUPS, groupLine, groupSizes, type MapGroup } from '@/state/materials';

const CODECS: SegmentedOption<TextureCodec>[] = [
  { value: 'etc1s', label: 'ETC1S' },
  { value: 'uastc', label: 'UASTC' },
];

export interface MapCodecRowsProps {
  model: InspectModel;
  /** Saves one group's codec; null while the row cannot change. */
  onCodec: ((group: MapGroup, codec: TextureCodec) => void) | null;
}

/**
 * The codec per map group (color, normal, data) with what each group's
 * maps weigh as built, so a run shows what a codec choice cost or saved.
 */
export default function MapCodecRows({ model, onCodec }: MapCodecRowsProps) {
  const sizes = groupSizes(model.textures);
  const rows = MAP_GROUPS.map(({ group, label, maps }) => {
    const codec = model.preset.codecs[group];
    const line = groupLine(sizes[group], codec);
    return (
      <div key={group} className="flex items-center justify-between gap-3">
        <div className="min-w-0">
          <div className="text-fg-dim" title={maps}>
            {label} maps
          </div>
          <div
            title={line.note}
            className={cn(
              'truncate font-mono text-[11.5px] leading-4',
              line.due ? 'text-clay-text' : 'text-fg-mute'
            )}
          >
            {line.text}
          </div>
        </div>
        <SegmentedControl
          label={`${label} map codec`}
          tone="clay"
          options={CODECS}
          value={codec}
          onChange={next => {
            if (onCodec && next !== codec) onCodec(group, next);
          }}
          disabled={!onCodec}
        />
      </div>
    );
  });
  return <div className="flex flex-col gap-2">{rows}</div>;
}
