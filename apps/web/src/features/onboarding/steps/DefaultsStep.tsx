import { Panel } from '@/features/common/Panel';
import { BudgetFields, CompressionFields, TripoFields } from '@/features/config/ConfigFields';
import { validateConfig } from '@/state/settings-form';
import type { OnboardingApi } from '../useOnboarding';

export function DefaultsStep({ api }: { api: OnboardingApi }) {
  const config = api.draft.config;
  const onChange = (next: typeof config) => api.update({ config: next });
  const errors = validateConfig(config);
  const props = { config, onChange, errors };
  return (
    <div className="mt-7 grid grid-cols-2 gap-4">
      <Panel
        title="Tripo preset"
        help="for every ref you send to Tripo"
        helpInline
        className="col-span-2"
      >
        <TripoFields {...props} />
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
