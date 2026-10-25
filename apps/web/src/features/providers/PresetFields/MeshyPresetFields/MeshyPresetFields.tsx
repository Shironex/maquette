import { NumberField, Select, Toggle, cn } from '@maquette/ui';
import type { MeshyPreset } from '@/ipc/client';
import { parseCount } from '@/features/config/parseCount';
import { MESHY_TEXTURE, POSE, TOPOLOGY } from '@/features/config/options';
import { formatK } from '@/lib/format';
import { isSmartTopology, meshyTakesRemoveLighting } from '@/state/provider-presets';
import { Group } from '../Group';
import type { PresetFieldsProps } from '../PresetFieldsProps';
import { modelOptions } from '../modelOptions';
import { useMeshyPresetFields } from './MeshyPresetFields.hooks';

export type MeshyPresetFieldsProps = PresetFieldsProps<MeshyPreset>;

export default function MeshyPresetFields({
  preset,
  onChange,
  variant,
  error,
  compact,
}: MeshyPresetFieldsProps) {
  const { catalog, smart, low, high } = useMeshyPresetFields(preset);
  const set = (patch: Partial<MeshyPreset>) => onChange({ ...preset, ...patch });
  const facesOn = smart || preset.shouldRemesh;
  return (
    <>
      <div className={cn('grid grid-cols-3', compact ? 'gap-2.5' : 'gap-3')}>
        <Select
          label="Model"
          options={modelOptions(catalog.meshyModels, preset.aiModel)}
          value={preset.aiModel}
          onChange={aiModel =>
            set({ aiModel, topology: isSmartTopology(aiModel) ? 'triangle' : preset.topology })
          }
        />
        <NumberField
          label={smart ? 'Faces' : 'Remesh to'}
          value={preset.targetPolycount}
          min={low}
          max={high}
          format={formatK}
          parse={parseCount}
          onChange={targetPolycount => set({ targetPolycount })}
          hint={facesOn ? `${formatK(low)} to ${formatK(high)}` : 'Turn on Remesh to set it'}
        />
        <Select
          label="Texture"
          options={MESHY_TEXTURE}
          value={preset.textureResolution}
          disabled={!preset.shouldTexture}
          onChange={textureResolution => set({ textureResolution })}
        />
      </div>
      <div className={cn('flex flex-wrap', compact ? 'gap-x-[22px] gap-y-2.5' : 'gap-x-6 gap-y-3')}>
        <Toggle
          label="Texture"
          checked={preset.shouldTexture}
          onChange={shouldTexture =>
            set({ shouldTexture, enablePbr: shouldTexture && preset.enablePbr })
          }
        />
        <Toggle
          label="PBR"
          checked={preset.enablePbr}
          disabled={!preset.shouldTexture}
          onChange={enablePbr => set({ enablePbr })}
        />
        <Toggle
          label="Remesh"
          checked={!smart && preset.shouldRemesh}
          disabled={smart}
          onChange={shouldRemesh => set({ shouldRemesh })}
        />
        <Toggle
          label="Remove lighting"
          checked={meshyTakesRemoveLighting(preset.aiModel) && preset.removeLighting}
          disabled={!meshyTakesRemoveLighting(preset.aiModel)}
          onChange={removeLighting => set({ removeLighting })}
        />
        <Toggle
          label="Multi-view"
          checked={preset.multiView}
          onChange={multiView => set({ multiView })}
        />
      </div>
      <div className={cn('grid grid-cols-2', compact ? 'gap-2.5' : 'gap-3')}>
        <Select
          label="Topology"
          options={TOPOLOGY}
          value={smart ? 'triangle' : preset.topology}
          disabled={smart}
          title={smart ? 'Smart Topology models make triangles' : undefined}
          onChange={topology => set({ topology })}
        />
        <Select
          label="Pose"
          options={POSE}
          value={preset.poseMode}
          onChange={poseMode => set({ poseMode })}
        />
      </div>
      {variant === 'character' ? (
        <Group title="Rig">
          <div className={cn('grid grid-cols-[auto_1fr] items-end', compact ? 'gap-2.5' : 'gap-3')}>
            <div className="pb-[7px]">
              <Toggle label="Rig" checked={preset.rig} onChange={rig => set({ rig })} />
            </div>
            <NumberField
              label="Character height"
              unit="m"
              value={preset.heightMeters ?? 0}
              min={0.1}
              max={20}
              format={n => n.toFixed(2)}
              onChange={heightMeters => set({ heightMeters })}
              hint="Meshy adds a walk and a run"
            />
          </div>
        </Group>
      ) : null}
      {error ? <p className="text-meta text-danger">{error}</p> : null}
    </>
  );
}
