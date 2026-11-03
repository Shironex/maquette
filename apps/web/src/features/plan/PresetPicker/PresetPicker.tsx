import { useId } from 'react';
import { Icon, cn } from '@maquette/ui';
import type { PlanPreset } from '@/ipc/client';
import { PRESETS } from '@/state/plan-presets';
import PresetArt from './PresetArt';
import PresetTag from './PresetTag';

export interface PresetPickerProps {
  value: PlanPreset;
  onChange: (preset: PlanPreset) => void;
  /** The group's visible label. */
  label?: string;
}

/**
 * Brainstorm, Story or Game mechanics as a radio group of cards: a small
 * picture of the start, the name with its tag, and a line on what it holds.
 */
export default function PresetPicker({ value, onChange, label = 'Start from' }: PresetPickerProps) {
  const id = useId();
  const cards = PRESETS.map(p => {
    const on = p.id === value;
    return (
      <label
        key={p.id}
        className={cn(
          'grid cursor-pointer grid-cols-[52px_minmax(0,1fr)_18px] items-center gap-3 rounded-card border bg-raised px-3 py-2.5 has-focus-visible:outline-2 has-focus-visible:outline-focus',
          on ? 'border-clay shadow-[0_0_0_1px_var(--clay)]' : 'border-line hover:border-line-strong'
        )}
      >
        <input
          type="radio"
          name={id}
          value={p.id}
          checked={on}
          onChange={() => onChange(p.id)}
          className="sr-only"
        />
        <span className="h-10 w-[52px] overflow-hidden rounded-[7px] border border-line bg-app">
          <PresetArt preset={p.id} />
        </span>
        <span className="flex min-w-0 flex-col gap-px">
          <span className="flex items-center gap-2 text-[13px] leading-[18px] font-semibold text-fg">
            {p.name}
            {p.tag ? <PresetTag>{p.tag}</PresetTag> : null}
          </span>
          <span className="text-[12px] leading-4 text-fg-mute">{p.blurb}</span>
        </span>
        <span
          aria-hidden="true"
          className={cn(
            'grid size-4 place-items-center rounded-full border-[1.5px]',
            on ? 'border-clay-text bg-clay text-clay-ink' : 'border-line-strong'
          )}
        >
          {on ? <Icon name="check" size={10} /> : null}
        </span>
      </label>
    );
  });
  return (
    <div className="flex flex-col gap-1.5">
      <span id={`${id}-label`} className="text-[12px] leading-4 font-medium text-fg-dim">
        {label}
      </span>
      <div role="radiogroup" aria-labelledby={`${id}-label`} className="flex flex-col gap-2">
        {cards}
      </div>
    </div>
  );
}
