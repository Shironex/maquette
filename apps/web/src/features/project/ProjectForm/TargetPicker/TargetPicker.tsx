import { Icon, cn, type IconName } from '@maquette/ui';
import type { ProjectTarget } from '@/ipc/client';

const TARGETS: readonly { id: ProjectTarget; title: string; detail: string; icon: IconName }[] = [
  {
    id: 'website',
    title: 'Website',
    detail: 'A web scene. Ship copies models, posters and scene data into a site repo.',
    icon: 'globe',
  },
  {
    id: 'game',
    title: 'Game',
    detail: 'Models for your own engine. Ship writes model files into the game repo.',
    icon: 'game-controller',
  },
];

export interface TargetPickerProps {
  value: ProjectTarget;
  compact?: boolean;
  onChange: (target: ProjectTarget) => void;
}

/** Where the project ships: a website or a game. */
export default function TargetPicker({ value, compact, onChange }: TargetPickerProps) {
  const cards = TARGETS.map(t => {
    const on = t.id === value;
    return (
      <button
        key={t.id}
        type="button"
        aria-pressed={on}
        onClick={() => onChange(t.id)}
        className={cn(
          'flex cursor-pointer items-start gap-3 rounded-card border bg-raised px-3 py-2.5 text-left transition-colors duration-120',
          on ? 'border-clay shadow-[0_0_0_1px_var(--clay)]' : 'border-line hover:border-line-strong'
        )}
      >
        <span
          className={cn(
            'grid size-8 flex-none place-items-center rounded-[9px]',
            on ? 'bg-clay-soft text-clay-text' : 'bg-hover text-fg-mute'
          )}
        >
          <Icon name={t.icon} size={18} />
        </span>
        <span className="min-w-0 flex-1">
          <span className="block font-semibold">{t.title}</span>
          <span className="block text-meta text-fg-mute">{t.detail}</span>
        </span>
        <span
          aria-hidden="true"
          className={cn(
            'mt-0.5 size-4 flex-none rounded-full border',
            on ? 'border-[5px] border-clay' : 'border-line-strong'
          )}
        />
      </button>
    );
  });
  return (
    <div className={compact ? '' : 'mt-7'}>
      <div id="ships-to" className="mb-1.5 text-[12px] leading-4 font-medium text-fg-dim">
        Ships to
      </div>
      <div role="group" aria-labelledby="ships-to" className="grid grid-cols-2 gap-3">
        {cards}
      </div>
    </div>
  );
}
