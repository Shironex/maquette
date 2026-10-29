/**
 * By hand on Tripo: the Tripo Studio tab and the rig the settings card asks
 * for. Multi-view has no switch here: it follows the images the send takes.
 */
import { Select, cn } from '@maquette/ui';
import type { StudioSettings } from '@/ipc/client';
import { STUDIO_MESH, STUDIO_RIG, studioMode } from '@/state/studio';

export interface StudioFieldsProps {
  value: StudioSettings;
  onChange: (value: StudioSettings) => void;
  /** How many images go, front first: two or more go as Multiview. */
  views: number;
  /** Names the controls for one asset of a batch. */
  name?: string;
  compact?: boolean;
}

export default function StudioFields({ value, onChange, views, name, compact }: StudioFieldsProps) {
  const of = name ? ` for ${name}` : '';
  return (
    <div className="flex flex-col gap-1.5">
      <div className={cn('grid grid-cols-2', compact ? 'gap-2.5' : 'gap-3')}>
        <Select
          label="Tripo Studio tab"
          aria-label={`Tripo Studio tab${of}`}
          options={STUDIO_MESH}
          value={value.mesh}
          onChange={mesh => onChange({ ...value, mesh })}
        />
        <Select
          label="Rig"
          aria-label={`Rig${of}`}
          options={STUDIO_RIG}
          value={value.rig}
          onChange={rig => onChange({ ...value, rig })}
        />
      </div>
      <p className="text-meta text-fg-mute">
        {views >= 2
          ? `${studioMode(views)}, each image in the slot of its view.`
          : 'Image to 3D from the front. Pick more views for Multiview.'}
      </p>
    </div>
  );
}
