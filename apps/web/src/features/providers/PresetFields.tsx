/**
 * The Tripo and Meshy preset controls. Settings edits the saved presets
 * with them; the Send to 3D panel edits a copy for one send.
 */
import { useState } from 'react';
import { NumberField, SegmentedControl, Select, Toggle, cn } from '@maquette/ui';
import type {
  MeshyPreset,
  ModelOption,
  PresetVariant,
  ProviderCatalog,
  ProviderId,
  TripoPreset,
  WorkspaceConfig,
} from '@/ipc/client';
import { parseCount } from '@/features/config/parseCount';
import {
  MESHY_TEXTURE,
  POSE,
  PRESET_VARIANTS,
  RIG_SPEC,
  SERVICES,
  TOPOLOGY,
  TRIPO_GEOMETRY,
  TRIPO_TEXTURE_QUALITY,
} from '@/features/config/options';
import { formatK } from '@/lib/format';
import { useAppStore } from '@/state/use-app-store';
import {
  isSmartTopology,
  isTripoPSeries,
  meshyPolycountRange,
  meshyTakesRemoveLighting,
  tripoFaceRange,
  type PresetKey,
} from '@/state/provider-presets';
import type { ConfigErrors } from '@/state/settings-form';

const EMPTY_CATALOG: ProviderCatalog = {
  tripoModels: [],
  tripoRigModels: [],
  tripoAnimations: [],
  meshyModels: [],
};

/** The model lists from the bootstrap. */
export function useCatalog(): ProviderCatalog {
  return useAppStore(s => s.boot?.catalog) ?? EMPTY_CATALOG;
}

/** Select options for a catalog list, keeping an unknown saved id visible. */
export function modelOptions(models: readonly ModelOption[], current: string) {
  const known = models.some(m => m.id === current);
  return [
    ...(known || !current ? [] : [{ value: current, label: current }]),
    ...models.map(m => ({ value: m.id, label: m.label })),
  ];
}

function FieldError({ text }: { text?: string }) {
  return text ? <p className="text-meta text-danger">{text}</p> : null;
}

/** 0 reads as "Adaptive"; the field also takes "auto". */
function formatFaces(n: number): string {
  return n === 0 ? 'Adaptive' : formatK(n);
}

function parseFaces(text: string): number | null {
  return /^\s*(adaptive|auto)?\s*$/i.test(text) ? 0 : parseCount(text);
}

interface PresetFieldsProps<P> {
  preset: P;
  onChange: (preset: P) => void;
  variant: PresetVariant;
  error?: string;
  compact?: boolean;
}

function Group({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-2.5 rounded-[8px] border border-line bg-raised px-3 py-2.5">
      <div className="font-semibold">{title}</div>
      {children}
    </div>
  );
}

function AnimationPicks({
  options,
  value,
  onChange,
}: {
  options: readonly ModelOption[];
  value: readonly string[];
  onChange: (value: string[]) => void;
}) {
  const full = value.length >= 5;
  return (
    <div role="group" aria-label="Animations" className="flex flex-wrap gap-1.5">
      {options.map(o => {
        const on = value.includes(o.id);
        return (
          <button
            key={o.id}
            type="button"
            aria-pressed={on}
            disabled={!on && full}
            title={!on && full ? 'Tripo retargets at most 5 at once' : undefined}
            onClick={() => onChange(on ? value.filter(v => v !== o.id) : [...value, o.id])}
            className={cn(
              'inline-flex h-[26px] cursor-pointer items-center rounded-control border border-line bg-app px-2.5 text-[12px] font-medium text-fg-dim transition-colors duration-120 hover:text-fg disabled:cursor-not-allowed disabled:opacity-45',
              on && 'border-clay-line bg-clay-soft text-fg'
            )}
          >
            {o.label}
          </button>
        );
      })}
    </div>
  );
}

export function TripoPresetFields({
  preset,
  onChange,
  variant,
  error,
  compact,
}: PresetFieldsProps<TripoPreset>) {
  const catalog = useCatalog();
  const set = (patch: Partial<TripoPreset>) => onChange({ ...preset, ...patch });
  const pSeries = isTripoPSeries(preset.model);
  const [low, high] = tripoFaceRange(preset);
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
      <FieldError text={error} />
    </>
  );
}

export function MeshyPresetFields({
  preset,
  onChange,
  variant,
  error,
  compact,
}: PresetFieldsProps<MeshyPreset>) {
  const catalog = useCatalog();
  const set = (patch: Partial<MeshyPreset>) => onChange({ ...preset, ...patch });
  const smart = isSmartTopology(preset.aiModel);
  const [low, high] = meshyPolycountRange(preset.aiModel);
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
      <FieldError text={error} />
    </>
  );
}

export interface ProviderPresetFieldsProps {
  config: WorkspaceConfig;
  onChange: (config: WorkspaceConfig) => void;
  errors?: ConfigErrors;
  density?: 'roomy' | 'compact';
}

/** The saved presets: pick a service and a variant, then edit it. */
export function ProviderPresetFields({
  config,
  onChange,
  errors = {},
  density = 'roomy',
}: ProviderPresetFieldsProps) {
  const [service, setService] = useState<ProviderId>('tripo');
  const [variant, setVariant] = useState<PresetVariant>('prop');
  const compact = density === 'compact';
  const key: PresetKey = `${service}-${variant}`;
  const flagged = (id: ProviderId, v: PresetVariant) => Boolean(errors[`${id}-${v}`]);
  const setProviders = (providers: WorkspaceConfig['providers']) =>
    onChange({ ...config, providers });

  return (
    <>
      <div className="flex flex-wrap items-center gap-2.5">
        <SegmentedControl
          label="Service"
          tone="clay"
          options={SERVICES.map(s => ({
            ...s,
            icon: PRESET_VARIANTS.some(v => flagged(s.value, v.value))
              ? ('warning-circle' as const)
              : undefined,
          }))}
          value={service}
          onChange={setService}
        />
        <SegmentedControl
          label="Preset"
          options={PRESET_VARIANTS.map(v => ({
            ...v,
            icon: flagged(service, v.value) ? ('warning-circle' as const) : undefined,
          }))}
          value={variant}
          onChange={setVariant}
        />
      </div>
      {service === 'tripo' ? (
        <TripoPresetFields
          key={key}
          preset={config.providers.tripo[variant]}
          variant={variant}
          compact={compact}
          error={errors[key]}
          onChange={preset =>
            setProviders({
              ...config.providers,
              tripo: { ...config.providers.tripo, [variant]: preset },
            })
          }
        />
      ) : (
        <MeshyPresetFields
          key={key}
          preset={config.providers.meshy[variant]}
          variant={variant}
          compact={compact}
          error={errors[key]}
          onChange={preset =>
            setProviders({
              ...config.providers,
              meshy: { ...config.providers.meshy, [variant]: preset },
            })
          }
        />
      )}
    </>
  );
}
