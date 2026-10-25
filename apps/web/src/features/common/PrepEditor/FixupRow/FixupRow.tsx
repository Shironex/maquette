import {
  IconButton,
  NumberField,
  SegmentedControl,
  Select,
  Tooltip,
  type SegmentedOption,
  type SelectOption,
} from '@maquette/ui';
import type { Fixup } from '@/ipc/client';
import { formatCount } from '@/lib/format';
import { FIXUP_HINT, FIXUP_LABEL } from '@/state/prepare';

const SIZES: SelectOption<string>[] = [256, 512, 1024, 2048, 4096].map(px => ({
  value: String(px),
  label: `${px} px`,
}));
const MERGE: SegmentedOption<'one' | 'material'>[] = [
  { value: 'one', label: 'One mesh' },
  { value: 'material', label: 'By material' },
];

export interface FixupRowProps {
  fixup: Fixup;
  /** Its place in the order they apply, from 1. */
  index: number;
  onChange: (fixup: Fixup) => void;
  onRemove: () => void;
  disabled?: boolean;
}

/** The number a fix-up takes, when it takes one. */
function Param({ fixup, onChange, disabled }: Omit<FixupRowProps, 'index' | 'onRemove'>) {
  switch (fixup.op) {
    case 'decimate':
      return (
        <NumberField
          aria-label="Triangle target"
          size="sm"
          value={fixup.triangles}
          min={100}
          max={10_000_000}
          format={formatCount}
          disabled={disabled}
          onChange={triangles => onChange({ ...fixup, triangles: Math.round(triangles) })}
          fieldClassName="w-[96px]"
        />
      );
    case 'shrink-textures':
      return (
        <Select
          aria-label="Largest texture"
          size="sm"
          inline
          options={SIZES}
          value={String(fixup.maxSize)}
          disabled={disabled}
          onChange={px => onChange({ ...fixup, maxSize: Number(px) })}
          fieldClassName="w-[96px]"
        />
      );
    case 'remove-loose':
      return (
        <NumberField
          aria-label="Largest part to remove"
          size="sm"
          unit="%"
          value={(fixup.minShare ?? 0) * 100}
          min={0}
          max={50}
          format={v => String(Number(v.toFixed(2)))}
          disabled={disabled}
          onChange={pct => onChange({ ...fixup, minShare: pct / 100 })}
          fieldClassName="w-[78px]"
        />
      );
    case 'merge':
      return (
        <SegmentedControl
          label="Merge into"
          options={MERGE}
          value={fixup.byMaterial ? 'material' : 'one'}
          onChange={v => onChange({ ...fixup, byMaterial: v === 'material' })}
        />
      );
    default:
      return null;
  }
}

/** One fix-up: its place, name and number, and Remove. */
export default function FixupRow({ fixup, index, onChange, onRemove, disabled }: FixupRowProps) {
  const label = FIXUP_LABEL[fixup.op];
  return (
    <li className="flex min-h-9 items-center gap-2 rounded-card bg-raised py-1 pr-1 pl-2.5">
      <span className="w-3 flex-none font-mono text-[11px] text-fg-mute tabular-nums">{index}</span>
      <Tooltip label={FIXUP_HINT[fixup.op]} wrap>
        <span className="min-w-0 flex-1 truncate">{label}</span>
      </Tooltip>
      <Param fixup={fixup} onChange={onChange} disabled={disabled} />
      <IconButton
        icon="x"
        size="sm"
        label={`Remove ${label.toLowerCase()}`}
        disabled={disabled}
        onClick={onRemove}
      />
    </li>
  );
}
