import { Tooltip, cn } from '@maquette/ui';
import type { ModelOption } from '@/ipc/client';

export interface AnimationPicksProps {
  options: readonly ModelOption[];
  value: readonly string[];
  onChange: (value: string[]) => void;
}

export default function AnimationPicks({ options, value, onChange }: AnimationPicksProps) {
  const full = value.length >= 5;
  const picks = options.map(o => {
    const on = value.includes(o.id);
    // Full keeps the rest focusable, so the tooltip can say why they do nothing.
    const blocked = !on && full;
    return (
      <Tooltip key={o.id} label="Tripo retargets at most 5 at once" disabled={!blocked}>
        <button
          type="button"
          aria-pressed={on}
          aria-disabled={blocked || undefined}
          onClick={() => {
            if (!blocked) onChange(on ? value.filter(v => v !== o.id) : [...value, o.id]);
          }}
          className={cn(
            'inline-flex h-[26px] cursor-pointer items-center rounded-control border border-line bg-app px-2.5 text-[12px] font-medium text-fg-dim transition-colors duration-120 hover:not-aria-disabled:text-fg aria-disabled:cursor-not-allowed aria-disabled:opacity-45',
            on && 'border-clay-line bg-clay-soft text-fg'
          )}
        >
          {o.label}
        </button>
      </Tooltip>
    );
  });
  return (
    <div role="group" aria-label="Animations" className="flex flex-wrap gap-1.5">
      {picks}
    </div>
  );
}
