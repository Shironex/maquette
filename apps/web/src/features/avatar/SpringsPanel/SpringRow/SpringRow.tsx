import { NumberStepper, Select, Toggle, cn } from '@maquette/ui';
import type { SpringCandidate, SpringPreset, SpringSpec } from '@/ipc/client';
import {
  JOINTS_MAX,
  JOINTS_MIN,
  KIND_LABEL,
  KIND_PRESET,
  PRESETS,
  confidenceLabel,
  lengthLabel,
} from '@/state/avatar-springs';
import { boneLabel } from '@/state/avatar';

export interface SpringRowProps {
  candidate: SpringCandidate;
  /** Its confirmed spring in the draft, or null. */
  spring: SpringSpec | null;
  disabled: boolean;
  onConfirm: (on: boolean) => void;
  onEdit: (change: Partial<Pick<SpringSpec, 'joints' | 'preset'>>) => void;
}

/**
 * One part the finder proposed: its colour on the sheet, what it is and
 * where it hangs from, how sure the finder is, and once confirmed, its
 * joints and how it swings.
 */
export default function SpringRow({
  candidate: c,
  spring,
  disabled,
  onConfirm,
  onEdit,
}: SpringRowProps) {
  const on = spring !== null;
  const unsure = (c.confidence ?? 0) < 0.6;
  const preset = spring?.preset ?? KIND_PRESET[c.kind];
  const hint = PRESETS.find(p => p.value === preset)?.hint;
  return (
    <li
      aria-label={`${c.id}, ${KIND_LABEL[c.kind].toLowerCase()} from the ${boneLabel(c.rootBone).toLowerCase()}`}
      className={cn(
        'flex flex-col gap-2.5 rounded-[9px] border px-3 py-2.5',
        on ? 'border-clay/45 bg-clay-soft/25' : 'border-line bg-raised'
      )}
    >
      <div className="flex items-center gap-3">
        <span
          aria-hidden="true"
          className="size-3.5 flex-none rounded-[4px] border border-black/15"
          style={{ backgroundColor: c.color }}
        />
        <div className="min-w-0 flex-1">
          <p className="flex items-baseline gap-2">
            <span className="font-mono text-[12.5px] font-medium">{c.id}</span>
            <span className="text-meta text-fg-dim">
              {KIND_LABEL[c.kind]} from the {boneLabel(c.rootBone).toLowerCase()}
            </span>
          </p>
          <p className="text-meta text-fg-mute tabular-nums">
            {lengthLabel(c.length ?? 0)} long, {c.vertices.toLocaleString('en-US')} vertices,{' '}
            <span className={unsure ? 'text-amber' : undefined}>
              {confidenceLabel(c.confidence ?? 0)} sure
            </span>
          </p>
        </div>
        <Toggle
          aria-label={`Swing ${c.id}`}
          checked={on}
          disabled={disabled}
          onChange={onConfirm}
        />
      </div>
      {spring ? (
        <div className="ml-6.5 flex flex-wrap items-center gap-4">
          <span className="flex items-center gap-2 text-meta text-fg-dim">
            Joints
            <NumberStepper
              label={`Joints in ${c.id}`}
              value={spring.joints ?? c.joints}
              min={JOINTS_MIN}
              max={JOINTS_MAX}
              onChange={joints => onEdit({ joints })}
            />
          </span>
          <Select<SpringPreset>
            aria-label={`How ${c.id} swings`}
            size="sm"
            inline
            label="Swing"
            options={PRESETS}
            value={preset}
            onChange={preset => onEdit({ preset })}
            fieldClassName="w-[150px]"
          />
          <span className="text-meta text-fg-mute">{hint}</span>
        </div>
      ) : null}
    </li>
  );
}
