import { Panel } from '@/features/common/Panel';
import { BudgetFields, CompressionFields } from '@/features/config/ConfigFields';
import { ProviderPresetFields, useCatalog } from '@/features/providers/PresetFields';
import { validateConfig } from '@/state/settings-form';
import type { OnboardingApi } from '../useOnboarding';

export function DefaultsStep({ api }: { api: OnboardingApi }) {
  const config = api.draft.config;
  const catalog = useCatalog();
  const onChange = (next: typeof config) => api.update({ config: next });
  const errors = validateConfig(config, catalog);
  const props = { config, onChange, errors };
  return (
    <div className="mt-7 grid grid-cols-2 gap-4">
      <Panel
        title="3D provider presets"
        help="what a Send to 3D starts from, changeable per send"
        helpInline
        className="col-span-2"
      >
        <ProviderPresetFields {...props} />
      </Panel>
      <Panel title="Web budgets" help="Over budget shows in red on the Board">
        <div className="grid grid-cols-2 gap-3">
          <BudgetFields {...props} />
        </div>
      </Panel>
      <Panel title="Compression" help="Applied by the Compress step">
        <div className="grid grid-cols-2 gap-3">
          <CompressionFields {...props} />
        </div>
      </Panel>
    </div>
  );
}
