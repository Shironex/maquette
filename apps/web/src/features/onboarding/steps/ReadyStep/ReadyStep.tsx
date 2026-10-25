import { Button, Icon, type IconName } from '@maquette/ui';
import type { ReadyRow } from '@/state/onboarding';
import { useReadyRows } from './ReadyStep.hooks';
import type { ReadyStepProps } from './ReadyStep.types';

const ICONS: Record<ReadyRow['id'], IconName> = {
  workspace: 'folder',
  toolchain: 'wrench',
  defaults: 'sliders-horizontal',
  agents: 'robot',
  project: 'cube',
};

export default function ReadyStep({ api }: ReadyStepProps) {
  const rows = useReadyRows(api).map(row => (
    <div
      key={row.id}
      className="flex items-start gap-3.5 border-t border-line py-[13px] first:border-t-0"
    >
      <span className="grid size-8 flex-none place-items-center rounded-[8px] bg-raised">
        <Icon name={ICONS[row.id]} className="text-fg-dim" />
      </span>
      <div className="w-[150px] flex-none pt-1.5 font-semibold">{row.title}</div>
      <div className="min-w-0 flex-1 pt-1.5 text-fg-dim">
        {row.mono ? (
          <span className="font-mono text-[12.5px] tracking-[-0.01em]">{row.mono} </span>
        ) : null}
        {row.text}
        {row.warning ? (
          <>
            {' '}
            <span className="text-amber">{row.warning}</span>
            {row.effect ? `, ${row.effect}` : null}
          </>
        ) : null}
      </div>
      <Button
        size="sm"
        variant="ghost"
        className="mt-[3px]"
        aria-label={`Edit ${row.title.toLowerCase()}`}
        onClick={() => api.setStep(row.step)}
      >
        Edit
      </Button>
    </div>
  ));
  return (
    <section
      aria-label="Setup summary"
      className="mt-7 rounded-card border border-line bg-panel px-4 py-1"
    >
      {rows}
    </section>
  );
}
