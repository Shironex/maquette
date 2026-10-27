import { Button, Icon, NumberField, VersionBadge } from '@maquette/ui';
import type { LayoutModel, Measured } from '@/ipc/client';
import { formatBytes } from '@/lib/format';
import { num } from '@/state/inspector';
import { MAX_SCALE, MIN_SCALE, type PlainLayout } from '@/state/layout';
import { fixed } from '../fixed';
import { sizeLine } from '../size-line';
import { Vec3Fields } from '../Vec3Fields';
import { useModelDetails } from './ModelDetails.hooks';

const MEASURED: Record<Measured, string> = {
  shipped: 'shipped build',
  raw: 'raw export',
  source: 'source model',
};

export interface ModelDetailsProps {
  model: LayoutModel;
  layout: PlainLayout;
}

/** The selected model's name, size and placement fields. */
export default function ModelDetails({ model, layout }: ModelDetailsProps) {
  const { p, place, dropToDesk } = useModelDetails(model, layout);
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
          onClick={dropToDesk}
          tooltip="Moves the model so its lowest point rests on the desk"
        >
          Drop to desk
        </Button>
        <Button
          size="sm"
          icon="arrow-counter-clockwise"
          tooltip="Sets the turn back to none and the scale back to 1"
          onClick={() => place({ rotation: [0, 0, 0], scale: 1 })}
        >
          Reset turn and scale
        </Button>
      </div>
    </div>
  );
}
