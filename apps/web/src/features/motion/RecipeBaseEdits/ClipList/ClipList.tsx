import { Button, IconButton, NumberField, SegmentedControl, Select, cn } from '@maquette/ui';
import type { BlendCurve, LineUp } from '@/ipc/client';
import { STATUS_WORDS } from '@/state/recipe-lanes';
import { DEFAULT_BLEND } from '@/state/recipe-v2-check';
import { useClipList } from './ClipList.hooks';

const CURVES = [
  { value: 'smooth', label: 'Smooth' },
  { value: 'linear', label: 'Linear' },
] as const satisfies readonly { value: BlendCurve; label: string }[];

const LINE_UPS = [
  { value: 'hips-and-facing', label: 'Hips and facing' },
  { value: 'hips', label: 'Hips' },
  { value: 'off', label: 'Off' },
] as const satisfies readonly { value: LineUp; label: string }[];

const STATUS_LOOK = {
  found: 'text-sage',
  'hash-differs': 'text-amber',
  missing: 'text-danger',
  rest: 'text-fg-mute',
} as const;

type ClipRow = ReturnType<typeof useClipList>['clips'][number];

/** The blend from the clip before into `row`'s clip: length, curve and line up. */
function BlendLine({
  row,
  onBlend,
}: {
  row: ClipRow;
  onBlend: ReturnType<typeof useClipList>['setBlend'];
}) {
  const blend = row.clip.blend ?? DEFAULT_BLEND;
  return (
    <div
      role="group"
      aria-label={`Blend into clip ${row.index + 1}`}
      className="ml-5 flex flex-col gap-2 border-l-2 border-clay/50 py-1 pl-3"
    >
      <div className="grid grid-cols-[88px_1fr] gap-2">
        <NumberField
          label="Length"
          size="sm"
          unit="f"
          min={0}
          max={Math.max(row.most, 0)}
          value={blend.frames}
          onChange={v => onBlend(row.index, { frames: Math.round(v) })}
        />
        <Select
          label="Line up"
          size="sm"
          options={LINE_UPS}
          value={blend.lineUp ?? 'hips-and-facing'}
          onChange={(v: LineUp) => onBlend(row.index, { lineUp: v })}
        />
      </div>
      <SegmentedControl
        label={`Blend curve into clip ${row.index + 1}`}
        options={CURVES}
        value={blend.curve ?? 'smooth'}
        onChange={(v: BlendCurve) => onBlend(row.index, { curve: v })}
      />
      <p className="text-meta text-fg-mute">
        {blend.frames === 0
          ? 'Cuts straight from one clip to the next.'
          : `The two clips overlap for ${blend.frames} f, at most ${row.most} f here.`}
      </p>
    </div>
  );
}

/** The recipe's clips in order: each with its state in the library, the blends between, move, take out and add. */
export default function ClipList() {
  const v = useClipList();
  const last = v.clips.length - 1;
  const rows = v.clips.map(row => (
    <li key={`${row.index}-${row.clip.name}`} className="flex flex-col gap-1.5">
      {row.index > 0 ? <BlendLine row={row} onBlend={v.setBlend} /> : null}
      <div className="flex items-center gap-2 rounded-[6px] border border-line bg-panel px-2 py-1.5">
        <span className="font-mono text-[11px] text-fg-mute">{row.index + 1}</span>
        <span className="min-w-0 flex-1">
          <span className="block truncate text-[12.5px] font-medium">{row.clip.name}</span>
          <span className="block truncate text-meta text-fg-mute">
            <span className={STATUS_LOOK[row.status]}>{STATUS_WORDS[row.status]}</span>
            {` · ${row.kept} f`}
            {row.clip.mixamo ? ' · Mixamo' : ''}
            {row.uses
              ? ` · ${row.uses === 1 ? '1 time edit' : `${row.uses} time edits`} on it`
              : ''}
          </span>
          {row.problem ? <span className="block text-meta text-danger">{row.problem}</span> : null}
        </span>
        <IconButton
          icon="arrow-up"
          size="sm"
          label={`Move ${row.clip.name} up`}
          disabled={row.index === 0}
          onClick={() => v.move(row.index, row.index - 1)}
        />
        <IconButton
          icon="arrow-down"
          size="sm"
          label={`Move ${row.clip.name} down`}
          disabled={row.index === last}
          onClick={() => v.move(row.index, row.index + 1)}
        />
        <IconButton
          icon="x"
          size="sm"
          label={`Take ${row.clip.name} out`}
          disabled={last === 0}
          className={cn(row.uses > 0 && 'opacity-70')}
          onClick={() => v.remove(row.index)}
        />
      </div>
    </li>
  ));
  const general = v.general.map(line => (
    <p key={line} role="alert" className="text-meta text-danger">
      {line}
    </p>
  ));
  return (
    <div
      role="group"
      aria-label="Clips settings"
      className="flex flex-col gap-2.5 border-t border-line px-3 py-3"
    >
      <ol aria-label="Clips in order" className="flex list-none flex-col gap-1.5">
        {rows}
      </ol>
      {v.problem ? (
        <p role="alert" className="text-meta text-danger">
          {v.problem}
        </p>
      ) : null}
      {general}
      <div>
        <Button size="sm" variant="ghost" icon="plus" disabled={!v.canAdd} onClick={v.add}>
          Add a clip
        </Button>
      </div>
    </div>
  );
}
