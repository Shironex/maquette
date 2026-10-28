import { IconButton, NumberField, Select, TextField } from '@maquette/ui';
import type { Pivot } from '@/ipc/client';
import { formatK } from '@/lib/format';
import { PIVOTS, type DraftPart } from '@/state/split';

export interface AssemblePartRowProps {
  part: DraftPart;
  index: number;
  count: number;
  /** The asset's name, for the row's labels. */
  assetName: string;
  byNumbers: boolean;
  onChange: (patch: Partial<DraftPart>) => void;
  onMove: (by: -1 | 1) => void;
  onRemove: () => void;
}

const AXES = ['x', 'y', 'z'] as const;

/** One part of the prop: its node name, pivot and, by numbers, its place. */
export default function AssemblePartRow({
  part,
  index,
  count,
  assetName,
  byNumbers,
  onChange,
  onMove,
  onRemove,
}: AssemblePartRowProps) {
  const at = part.at ?? [0, 0, 0];
  const setAxis = (axis: number, value: number) =>
    onChange({ at: at.map((v, i) => (i === axis ? value : (v ?? 0))) as [number, number, number] });
  const axes = AXES.map((axis, i) => (
    <NumberField
      key={axis}
      aria-label={`${part.name} ${axis}`}
      size="sm"
      prefix={axis}
      unit="m"
      value={at[i] ?? 0}
      onChange={v => setAxis(i, v)}
    />
  ));
  return (
    <li className="flex flex-col gap-2 rounded-card border border-line bg-raised px-3 py-2.5">
      <div className="flex items-center gap-2">
        <span className="w-4 flex-none text-center font-mono text-meta text-fg-mute">
          {index + 1}
        </span>
        <TextField
          aria-label={`Node name for ${assetName}`}
          mono
          value={part.name}
          onChange={e => onChange({ name: e.target.value })}
          fieldClassName="min-w-0 flex-1"
        />
        <IconButton
          icon="arrow-up"
          size="sm"
          label={`Move ${part.name} up`}
          disabled={index === 0}
          onClick={() => onMove(-1)}
        />
        <IconButton
          icon="arrow-down"
          size="sm"
          label={`Move ${part.name} down`}
          disabled={index === count - 1}
          onClick={() => onMove(1)}
        />
        <IconButton icon="x" size="sm" label={`Take out ${part.name}`} onClick={onRemove} />
      </div>
      <div className="flex items-center gap-2 pl-6">
        <span className="min-w-0 flex-1 truncate text-meta text-fg-mute">
          {assetName}
          {part.version ? ` v${part.version}` : ', newest version'} · {formatK(part.triangles)} tris
        </span>
        <Select<Pivot>
          aria-label={`Pivot of ${part.name}`}
          size="sm"
          options={PIVOTS}
          value={part.pivot}
          onChange={pivot => onChange({ pivot })}
        />
      </div>
      {byNumbers ? (
        <div className="grid grid-cols-4 gap-2 pl-6">
          {axes}
          <NumberField
            aria-label={`${part.name} turn`}
            size="sm"
            unit="°"
            value={part.turn ?? 0}
            onChange={turn => onChange({ turn })}
          />
        </div>
      ) : null}
    </li>
  );
}
