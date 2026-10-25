import { useId } from 'react';
import { NumberField } from '@maquette/ui';
import type { Vec3 } from '@/state/inspector';
import { fixed } from '../fixed';

const AXES = ['X', 'Y', 'Z'] as const;

export interface Vec3FieldsProps {
  label: string;
  value: Vec3;
  unit: string;
  digits: number;
  onChange: (value: Vec3) => void;
}

/** Three number fields, X, Y and Z, edited as one value. */
export default function Vec3Fields({ label, value, unit, digits, onChange }: Vec3FieldsProps) {
  const id = useId();
  const fields = AXES.map((axis, i) => (
    <div key={axis} className="flex min-w-0 items-center gap-1.5">
      <label htmlFor={`${id}-${axis}`} className="w-2 flex-none text-[11px] text-fg-mute">
        {axis}
      </label>
      <NumberField
        id={`${id}-${axis}`}
        size="sm"
        value={value[i]!}
        format={fixed(digits)}
        fieldClassName="min-w-0 flex-1"
        onChange={v => {
          const next = [...value] as Vec3;
          next[i] = v;
          onChange(next);
        }}
      />
    </div>
  ));
  return (
    <div role="group" aria-label={`${label} in ${unit}`} className="flex flex-col gap-1.5">
      <span aria-hidden="true" className="text-[12px] leading-4 font-medium text-fg-dim">
        {label} <span className="font-normal text-fg-mute">{unit}</span>
      </span>
      <div className="grid min-w-0 grid-cols-3 gap-1.5">{fields}</div>
    </div>
  );
}
