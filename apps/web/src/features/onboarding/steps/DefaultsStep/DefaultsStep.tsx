import { Button, cn } from '@maquette/ui';
import { Panel } from '@/features/common/Panel';
import { BudgetFields } from '@/features/config/BudgetFields';
import { CompressionFields } from '@/features/config/CompressionFields';
import { ProviderPresetFields, useCatalog } from '@/features/providers/PresetFields';
import { defaultsSummary } from '@/state/onboarding';
import { validateConfig } from '@/state/settings-form';
import { useDefaultsStep } from './DefaultsStep.hooks';
import type { DefaultsStepProps } from './DefaultsStep.types';

export default function DefaultsStep({ api }: DefaultsStepProps) {
  const config = api.draft.config;
  const catalog = useCatalog();
  const onChange = (next: typeof config) => api.update({ config: next });
  const errors = validateConfig(config, catalog);
  const props = { config, onChange, errors };
  const { editing, edit, recommended, routes } = useDefaultsStep(config);
  const chips = routes.map(route => (
    <li
      key={route.name}
      title={route.detail}
      className="flex items-center gap-1.5 rounded-pill border border-line bg-raised px-2.5 py-1 text-meta"
    >
      <span
        aria-hidden
        className={cn('size-1.5 rounded-full', route.ready ? 'bg-sage' : 'bg-fg-mute')}
      />
      <span className="font-medium">{route.name}</span>
      <span className="text-fg-mute">{route.status}</span>
    </li>
  ));
  return (
    <>
      <section aria-label="Ways to make models" className="mt-7">
        <h2 className="text-[13px] font-semibold">Ways to make models</h2>
        <ul className="mt-2 flex list-none flex-wrap gap-2 p-0">{chips}</ul>
      </section>
      {editing ? (
        <div className="mt-5 grid grid-cols-2 gap-4">
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
          <Button
            size="sm"
            className="col-span-2 justify-self-start"
            icon="arrow-clockwise"
            tooltip="Puts every default back to what Maquette recommends"
            onClick={() => onChange(recommended())}
          >
            Use recommended
          </Button>
        </div>
      ) : (
        <section
          aria-label="Recommended defaults"
          className="mt-5 flex items-start gap-3 rounded-card border border-clay-line bg-panel p-4"
        >
          <div className="min-w-0 flex-1">
            <h2 className="text-[13px] font-semibold">Use recommended</h2>
            <p className="mt-1 text-meta text-fg-dim">{defaultsSummary(config, catalog)}</p>
          </div>
          <Button size="sm" tooltip="Shows every preset, budget and setting" onClick={edit}>
            Edit
          </Button>
        </section>
      )}
    </>
  );
}
