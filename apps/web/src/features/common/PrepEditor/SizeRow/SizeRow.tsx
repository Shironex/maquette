import {
  IconButton,
  NumberField,
  SegmentedControl,
  Select,
  type SegmentedOption,
  type SelectOption,
} from '@maquette/ui';
import type { RealSize, SizeAxis, SizeUnit } from '@/ipc/client';
import { AXIS_NAME } from '@/state/prepare';

const AXES: SelectOption<SizeAxis>[] = (['height', 'width', 'depth', 'longest'] as const).map(
  value => ({ value, label: AXIS_NAME[value] })
);
const UNITS: SegmentedOption<SizeUnit>[] = [
  { value: 'cm', label: 'cm' },
  { value: 'm', label: 'm' },
];
/** The largest value per unit: 1000 m. */
const MAX: Record<SizeUnit, number> = { cm: 100_000, m: 1000 };

export interface SizeRowProps {
  size: RealSize;
  onChange: (size: RealSize) => void;
  onClear: () => void;
  disabled?: boolean;
}

/** The side, the length and the unit of a real size; a new unit keeps the length. */
export default function SizeRow({ size, onChange, onClear, disabled }: SizeRowProps) {
  const value = size.value ?? 0;
  return (
    <div className="flex items-end gap-2">
      <Select
        label="Side"
        size="sm"
        options={AXES}
        value={size.axis}
        disabled={disabled}
        onChange={axis => onChange({ ...size, axis })}
        fieldClassName="w-[118px]"
      />
      <NumberField
        label="Length"
        size="sm"
        value={value}
        min={size.unit === 'cm' ? 0.1 : 0.001}
        max={MAX[size.unit]}
        disabled={disabled}
        format={v => String(Number(v.toFixed(3)))}
        onChange={next => onChange({ ...size, value: next })}
        fieldClassName="w-[78px]"
      />
      <SegmentedControl
        label="Unit"
        tone="clay"
        options={UNITS}
        value={size.unit}
        onChange={unit => {
          if (unit === size.unit) return;
          const next = unit === 'm' ? value / 100 : value * 100;
          onChange({ ...size, unit, value: Number(next.toFixed(3)) });
        }}
      />
      <IconButton
        icon="x"
        size="sm"
        label="Clear the real size"
        hint="The model keeps the size it came at"
        disabled={disabled}
        onClick={onClear}
        className="ml-auto"
      />
    </div>
  );
}
