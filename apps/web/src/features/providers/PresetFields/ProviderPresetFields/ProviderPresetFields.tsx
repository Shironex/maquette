import { SegmentedControl } from '@maquette/ui';
import type { PresetVariant, ProviderId, WorkspaceConfig } from '@/ipc/client';
import { PRESET_VARIANTS, SERVICES } from '@/features/config/options';
import type { PresetKey } from '@/state/provider-presets';
import type { ConfigErrors } from '@/state/settings-form';
import { MeshyPresetFields } from '../MeshyPresetFields';
import { TripoPresetFields } from '../TripoPresetFields';
import { usePresetPick } from './ProviderPresetFields.hooks';

export interface ProviderPresetFieldsProps {
  config: WorkspaceConfig;
  onChange: (config: WorkspaceConfig) => void;
  errors?: ConfigErrors;
  density?: 'roomy' | 'compact';
}

/** The saved presets: pick a service and a variant, then edit it. */
export default function ProviderPresetFields({
  config,
  onChange,
  errors = {},
  density = 'roomy',
}: ProviderPresetFieldsProps) {
  const { service, setService, variant, setVariant } = usePresetPick();
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
