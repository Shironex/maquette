import { Icon, SegmentedControl, TextField } from '@maquette/ui';
import type { GameFormat } from '@/ipc/client';
import type { ProjectFields } from '@/state/onboarding';

export interface GameFieldsProps {
  value: ProjectFields;
  size: 'md' | 'lg';
  onChange: (patch: Partial<ProjectFields>) => void;
}

/** The model format and the model folder of a game, as the repo check found them. */
export default function GameFields({ value, size, onChange }: GameFieldsProps) {
  const detected = value.game?.format ?? null;
  const tag = (format: GameFormat) => (detected === format ? ' (detected)' : '');
  const formats = [
    { value: 'glb' as const, label: `glTF binary .glb${tag('glb')}` },
    { value: 'obj' as const, label: `Wavefront .obj + .mtl${tag('obj')}` },
  ];
  return (
    <div className="mt-4 grid grid-cols-[minmax(0,2fr)_minmax(0,1fr)] items-end gap-3.5">
      <div className="flex min-w-0 flex-col gap-1.5">
        <span aria-hidden="true" className="text-[12px] leading-4 font-medium text-fg-dim">
          Model format
        </span>
        <SegmentedControl
          label="Model format"
          tone="clay"
          block
          options={formats}
          value={value.gameFormat}
          onChange={gameFormat => onChange({ gameFormat })}
        />
      </div>
      <TextField
        label="Model folder"
        size={size}
        value={value.modelsDir}
        onChange={e => onChange({ modelsDir: e.target.value })}
      />
      <p className="col-span-2 flex items-center gap-1.5 text-meta text-fg-mute">
        <Icon name="info" size={14} />
        Game projects skip Capture posters and camera stops. Bake light starts off.
      </p>
    </div>
  );
}
