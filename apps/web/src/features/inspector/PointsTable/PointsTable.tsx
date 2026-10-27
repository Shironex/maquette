import { Button, IconButton, NumberField, Tooltip, cn } from '@maquette/ui';
import type { LayoutModel, ScenePath } from '@/ipc/client';
import { MAX_POINTS, along, pointsOf } from '@/state/paths';
import type { Vec3 } from '@/state/inspector';
import { SectionTitle } from '../SectionTitle';
import { fixed } from '../LayoutPanel/fixed';
import { usePointsTable } from './PointsTable.hooks';

const AXES = ['X', 'Y', 'Z'] as const;
const axisHeads = AXES.map(axis => (
  <th key={axis} className="pb-1 font-medium">
    {axis}
  </th>
));

export interface PointsTableProps {
  path: ScenePath;
  models: LayoutModel[];
}

/** The selected path's points: where each is, how far along, and which end follows a model. */
export default function PointsTable({ path, models }: PointsTableProps) {
  const { picked, pick, move, add, remove } = usePointsTable(path);
  const points = pointsOf(path);
  const distances = along(path);
  const name = (asset: string | null) =>
    asset ? (models.find(m => m.asset === asset)?.name ?? asset) : null;
  const ends = [name(path.start), name(path.end)];
  const axisCells = (point: Vec3, i: number) =>
    AXES.map((axis, a) => (
      <td key={axis} className="py-1 pr-1.5">
        <NumberField
          aria-label={`Point ${i + 1} ${axis}`}
          size="sm"
          value={point[a]!}
          format={fixed(2)}
          fieldClassName="w-[84px]"
          onChange={v => {
            const next = [...point] as Vec3;
            next[a] = v;
            move(i, next);
          }}
        />
      </td>
    ));
  const rows = points.map((point, i) => {
    const on = picked === i;
    const anchor = i === 0 ? ends[0] : i === points.length - 1 ? ends[1] : null;
    return (
      <tr key={i} aria-selected={on} className={cn('border-t border-line', on && 'bg-clay-soft')}>
        <td className="py-1 pr-2">
          <Tooltip label="Selects this point in the view">
            <button
              type="button"
              aria-label={`Pick point ${i + 1}`}
              aria-pressed={on}
              onClick={() => pick(i)}
              className={cn(
                'grid size-6 cursor-pointer place-items-center rounded-[5px] font-mono text-[11px] hover:bg-hover',
                on ? 'text-clay-text' : 'text-fg-mute'
              )}
            >
              {i + 1}
            </button>
          </Tooltip>
        </td>
        {axisCells(point, i)}
        <td className="py-1 pr-2 text-right font-mono text-[11.5px] text-fg-mute">
          {distances[i]!.toFixed(2)} m
        </td>
        <td className="max-w-[160px] truncate py-1 pr-2 text-meta text-fg-dim">
          {anchor ? `Follows ${anchor}` : ''}
        </td>
        <td className="py-1 text-right">
          <IconButton
            icon="trash"
            size="sm"
            label={`Remove point ${i + 1}`}
            disabled={points.length <= 2}
            disabledReason="A path keeps at least two points"
            onClick={() => remove(i)}
          />
        </td>
      </tr>
    );
  });
  return (
    <section aria-label="Points" className="flex flex-col gap-2">
      <SectionTitle aside={`${points.length} of ${MAX_POINTS} · metres, y up`}>Points</SectionTitle>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[12.5px]">
          <thead>
            <tr className="text-left text-[11px] text-fg-mute">
              <th className="pb-1 font-medium">#</th>
              {axisHeads}
              <th className="pb-1 pr-2 text-right font-medium">Along</th>
              <th className="pb-1 font-medium">Anchor</th>
              <th className="pb-1" />
            </tr>
          </thead>
          <tbody>{rows}</tbody>
        </table>
      </div>
      <Button
        size="sm"
        icon="plus"
        className="self-start"
        disabled={points.length >= MAX_POINTS}
        disabledReason={`A path keeps ${MAX_POINTS} points at most`}
        tooltip="Adds a point after the last one"
        onClick={add}
      >
        Add point at the end
      </Button>
    </section>
  );
}
