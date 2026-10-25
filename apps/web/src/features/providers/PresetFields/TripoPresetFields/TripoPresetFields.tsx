import { NumberField, Select, Toggle, cn } from '@maquette/ui';
import type { TripoPreset } from '@/ipc/client';
import { RIG_SPEC, TRIPO_GEOMETRY, TRIPO_TEXTURE_QUALITY } from '@/features/config/options';
import { formatK } from '@/lib/format';
import { Group } from '../Group';
import type { PresetFieldsProps } from '../PresetFieldsProps';
import { modelOptions } from '../modelOptions';
import { AnimationPicks } from './AnimationPicks';
import { formatFaces, parseFaces } from './faces';
import { useTripoPresetFields } from './TripoPresetFields.hooks';

export type TripoPresetFieldsProps = PresetFieldsProps<TripoPreset>;

export default function TripoPresetFields({
  preset,
  onChange,
  variant,
  error,
  compact,
}: TripoPresetFieldsProps) {
  const { catalog, pSeries, low, high } = useTripoPresetFields(preset);
  const set = (patch: Partial<TripoPreset>) => onChange({ ...preset, ...patch });
  return (
    <>
      <div className={cn('grid grid-cols-3', compact ? 'gap-2.5' : 'gap-3')}>
        <Select
          label="Model"
          options={modelOptions(catalog.tripoModels, preset.model)}
          value={preset.model}
          onChange={model => set({ model })}
        />
        <NumberField
          label="Face limit"
          value={preset.faceLimit}
          min={0}
          max={high}
          format={formatFaces}
          parse={parseFaces}
          onChange={faceLimit => set({ faceLimit })}
          hint={`${formatK(low)} to ${formatK(high)}, or adaptive`}
        />
        <Select
          label="Texture quality"
          options={TRIPO_TEXTURE_QUALITY}
          value={preset.textureQuality}
          onChange={textureQuality => set({ textureQuality })}
          disabled={!preset.texture}
        />
      </div>
      <div className={cn('flex flex-wrap', compact ? 'gap-x-[22px] gap-y-2.5' : 'gap-x-6 gap-y-3')}>
        <Toggle
          label="Texture"
          checked={preset.texture}
          onChange={texture => set({ texture, pbr: texture && preset.pbr })}
        />
        <Toggle
          label="PBR"
          checked={preset.pbr}
          disabled={!preset.texture}
          onChange={pbr => set({ pbr })}
        />
        <Toggle
          label="Multi-view"
          checked={preset.multiView}
          onChange={multiView => set({ multiView })}
        />
        <Toggle
          label="Real-world size"
          checked={preset.autoSize}
          onChange={autoSize => set({ autoSize })}
        />
        <Toggle
          label="Fix the image first"
          checked={preset.imageAutofix}
          onChange={imageAutofix => set({ imageAutofix })}
        />
      </div>
      <div className={cn('grid grid-cols-2', compact ? 'gap-2.5' : 'gap-3')}>
        <Select
          label="Geometry"
          options={TRIPO_GEOMETRY}
          value={pSeries ? 'standard' : preset.geometryQuality}
          disabled={pSeries}
          title={pSeries ? 'The P series has one geometry level' : undefined}
          onChange={geometryQuality => set({ geometryQuality })}
        />
        <div className="flex items-end pb-[7px]">
          <Toggle
            label="Smart low poly"
            checked={!pSeries && preset.smartLowPoly}
            disabled={pSeries}
            onChange={smartLowPoly => set({ smartLowPoly })}
          />
        </div>
      </div>
      {variant === 'character' ? (
        <Group title="Rig and animate">
          <div
            className={cn('grid grid-cols-[auto_1fr_1fr] items-end', compact ? 'gap-2.5' : 'gap-3')}
          >
            <div className="pb-[7px]">
              <Toggle label="Rig" checked={preset.rig} onChange={rig => set({ rig })} />
            </div>
            <Select
              label="Rig model"
              options={modelOptions(catalog.tripoRigModels, preset.rigModel)}
              value={preset.rigModel}
              disabled={!preset.rig}
              onChange={rigModel => set({ rigModel })}
            />
            <Select
              label="Bones"
              options={RIG_SPEC}
              value={preset.rigSpec}
              disabled={!preset.rig}
              onChange={rigSpec => set({ rigSpec })}
            />
          </div>
          {preset.rig ? (
            <div className="flex flex-col gap-1.5">
              <span className="text-meta font-medium text-fg-dim">
                Animations <span className="text-fg-mute">up to 5</span>
              </span>
              <AnimationPicks
                options={catalog.tripoAnimations}
                value={preset.animations}
                onChange={animations => set({ animations })}
              />
            </div>
          ) : null}
        </Group>
      ) : null}
      {error ? <p className="text-meta text-danger">{error}</p> : null}
    </>
  );
}
